use crate::{Error, Journal, Result, Revision};
use notes_model::{ContentHash, NoteId, RelPath};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairingMode {
    Upload,
    Download,
    Reconcile,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Disabled,
    Offline,
    Pending,
    Syncing,
    Current,
    Conflict,
    Error,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct File {
    pub note: NoteId,
    pub path: RelPath,
    pub content: ContentHash,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PairingAction {
    Upload { local: File },
    Download { remote: File },
    Link { local: File, remote: File },
    Conflict { local: File, remote: File },
}
/// Initial pairing is a preview. An upload cannot replace a populated remote,
/// and a download cannot replace a populated local folder.
pub fn pair(mode: PairingMode, local: &[File], remote: &[File]) -> Result<Vec<PairingAction>> {
    fn indexed(files: &[File]) -> Result<BTreeMap<RelPath, File>> {
        let mut paths = BTreeMap::new();
        let mut ids = BTreeSet::new();
        for file in files {
            if !file.path.is_note() || !ids.insert(file.note) {
                return Err(Error::InvalidState);
            }
            if paths.insert(file.path.clone(), file.clone()).is_some() {
                return Err(Error::Collision);
            }
        }
        Ok(paths)
    }
    if (mode == PairingMode::Upload && !remote.is_empty())
        || (mode == PairingMode::Download && !local.is_empty())
    {
        return Err(Error::Pairing);
    }
    let local = indexed(local)?;
    let remote = indexed(remote)?;
    let paths: BTreeSet<_> = local.keys().chain(remote.keys()).collect();
    Ok(paths
        .into_iter()
        .map(|path| match (local.get(path), remote.get(path)) {
            (Some(a), Some(b)) if a.content == b.content => PairingAction::Link {
                local: a.clone(),
                remote: b.clone(),
            },
            (Some(a), Some(b)) => PairingAction::Conflict {
                local: a.clone(),
                remote: b.clone(),
            },
            (Some(a), None) => PairingAction::Upload { local: a.clone() },
            (None, Some(b)) => PairingAction::Download { remote: b.clone() },
            _ => unreachable!(),
        })
        .collect())
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    PathCollision {
        path: RelPath,
        local: NoteId,
        remote: NoteId,
    },
    Push {
        note: NoteId,
        expected: Option<Uuid>,
        revision: Uuid,
    },
    Pull {
        note: NoteId,
        expected: Option<Uuid>,
        revision: Uuid,
    },
    MergeEqual {
        note: NoteId,
        local: Uuid,
        remote: Uuid,
    },
    Conflict {
        note: NoteId,
        local: Uuid,
        remote: Uuid,
    },
}
/// Both sides must share the explicitly paired workspace, but retain separate
/// heads. An absent head is never an inferred deletion: only tombstones delete.
pub fn plan(local: &Journal, remote: &Journal) -> Result<Vec<Action>> {
    local.validate()?;
    remote.validate()?;
    if local.workspace != remote.workspace {
        return Err(Error::Pairing);
    }
    let mut graph = local.clone();
    graph.import(remote.revisions.values().cloned())?;
    let notes: BTreeSet<_> = local
        .heads
        .keys()
        .chain(remote.heads.keys())
        .copied()
        .collect();
    let mut actions = vec![];
    let mut blocked = BTreeSet::new();
    let local_paths: BTreeMap<_, _> = local
        .heads
        .keys()
        .filter_map(|n| {
            local
                .head(*n)
                .filter(|r| r.content.is_some())
                .map(|r| (r.path.clone(), r.note))
        })
        .collect();
    for note in remote.heads.keys() {
        let remote_head = remote.head(*note).ok_or(Error::InvalidGraph)?;
        if remote_head.content.is_none() {
            continue;
        }
        if let Some(local_note) = local_paths.get(&remote_head.path).filter(|n| **n != *note) {
            blocked.insert(*local_note);
            blocked.insert(*note);
            actions.push(Action::PathCollision {
                path: remote_head.path.clone(),
                local: *local_note,
                remote: *note,
            });
        }
    }
    for note in notes {
        if blocked.contains(&note) {
            continue;
        }
        match (
            local.heads.get(&note).copied(),
            remote.heads.get(&note).copied(),
        ) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) if graph.is_ancestor(b, a) => actions.push(Action::Push {
                note,
                expected: Some(b),
                revision: a,
            }),
            (Some(a), Some(b)) if graph.is_ancestor(a, b) => actions.push(Action::Pull {
                note,
                expected: Some(a),
                revision: b,
            }),
            (Some(a), Some(b)) if graph.revisions[&a].same_value(&graph.revisions[&b]) => actions
                .push(Action::MergeEqual {
                    note,
                    local: a,
                    remote: b,
                }),
            (Some(a), Some(b)) => actions.push(Action::Conflict {
                note,
                local: a,
                remote: b,
            }),
            (Some(a), None) => actions.push(Action::Push {
                note,
                expected: None,
                revision: a,
            }),
            (None, Some(b)) => actions.push(Action::Pull {
                note,
                expected: None,
                revision: b,
            }),
            (None, None) => unreachable!(),
        }
    }
    Ok(actions)
}
/// Explicit conflict resolution records both parents. The caller must still
/// compare-and-set each peer's observed head when applying the result.
pub fn resolve(
    graph: &Journal,
    local: Uuid,
    remote: Uuid,
    device: Uuid,
    path: RelPath,
    content: Option<ContentHash>,
) -> Result<Revision> {
    let a = graph.revisions.get(&local).ok_or(Error::InvalidGraph)?;
    let b = graph.revisions.get(&remote).ok_or(Error::InvalidGraph)?;
    if a.note != b.note || local == remote || !path.is_note() {
        return Err(Error::InvalidGraph);
    }
    Ok(Revision::new(
        a.note,
        BTreeSet::from([local, remote]),
        device,
        path,
        content,
    ))
}
