use notes_model::{ContentHash, NoteId, RelPath};
use notes_sync::{
    pair, plan, resolve, Action, Error, File, Journal, PairingAction, PairingMode, Revision,
};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;
fn p(path: &str) -> RelPath {
    RelPath::parse(path).unwrap()
}
fn h(text: &str) -> ContentHash {
    ContentHash::from_bytes(*blake3::hash(text.as_bytes()).as_bytes())
}
fn initial() -> (Journal, NoteId, Uuid) {
    let mut j = Journal::new(Uuid::new_v4());
    let note = NoteId::new();
    let r = Revision::new(
        note,
        BTreeSet::new(),
        Uuid::new_v4(),
        p("note.md"),
        Some(h("base")),
    );
    let id = r.id;
    j.commit(r, None).unwrap();
    (j, note, id)
}
fn edit(j: &mut Journal, note: NoteId, path: &str, text: Option<&str>) -> Uuid {
    let base = j.heads.get(&note).copied();
    let r = Revision::new(
        note,
        base.into_iter().collect(),
        Uuid::new_v4(),
        p(path),
        text.map(h),
    );
    let id = r.id;
    j.commit(r, base).unwrap();
    id
}
#[test]
fn edits_flow_by_ancestry_in_both_directions() {
    let (mut local, note, base) = initial();
    let remote = local.clone();
    let next = edit(&mut local, note, "note.md", Some("edited"));
    assert_eq!(
        plan(&local, &remote).unwrap(),
        vec![Action::Push {
            note,
            expected: Some(base),
            revision: next
        }]
    );
    assert_eq!(
        plan(&remote, &local).unwrap(),
        vec![Action::Pull {
            note,
            expected: Some(base),
            revision: next
        }]
    );
}
#[test]
fn equal_bytes_do_not_erase_causal_history() {
    let (mut a, note, base) = initial();
    let mut b = a.clone();
    let left = edit(&mut a, note, "note.md", Some("same"));
    let right = edit(&mut b, note, "note.md", Some("same"));
    assert_eq!(
        plan(&a, &b).unwrap(),
        vec![Action::MergeEqual {
            note,
            local: left,
            remote: right
        }]
    );
    a.import(b.revisions.values().cloned()).unwrap();
    let resolution = resolve(
        &a,
        left,
        right,
        Uuid::new_v4(),
        p("note.md"),
        Some(h("same")),
    )
    .unwrap();
    let merged = resolution.id;
    a.commit(resolution.clone(), Some(left)).unwrap();
    b.import(a.revisions.values().cloned()).unwrap();
    b.advance(note, Some(right), merged).unwrap();
    assert!(a.is_ancestor(base, merged));
    assert!(a.is_ancestor(left, merged));
    assert!(a.is_ancestor(right, merged));
    assert!(plan(&a, &b).unwrap().is_empty());
}
#[test]
fn edit_delete_and_rename_edit_are_conflicts_without_a_clock_winner() {
    for (path, text) in [("note.md", None), ("renamed.md", Some("base"))] {
        let (mut a, note, _) = initial();
        let mut b = a.clone();
        let left = edit(&mut a, note, path, text);
        let right = edit(&mut b, note, "note.md", Some("offline edit"));
        assert_eq!(
            plan(&a, &b).unwrap(),
            vec![Action::Conflict {
                note,
                local: left,
                remote: right
            }]
        );
        assert_eq!(a.head(note).unwrap().content, text.map(h));
        assert_eq!(b.head(note).unwrap().content, Some(h("offline edit")));
    }
}
#[test]
fn tombstones_propagate_and_ordinary_absence_never_means_deletion() {
    let (mut local, note, base) = initial();
    let remote = local.clone();
    let deleted = edit(&mut local, note, "note.md", None);
    assert_eq!(
        plan(&local, &remote).unwrap(),
        vec![Action::Push {
            note,
            expected: Some(base),
            revision: deleted
        }]
    );
    let empty = Journal::new(local.workspace);
    assert_eq!(
        plan(&empty, &remote).unwrap(),
        vec![Action::Pull {
            note,
            expected: None,
            revision: base
        }]
    );
    assert_eq!(local.revisions.len(), 2);
}
#[test]
fn stale_writes_and_stale_resolution_leave_heads_unchanged() {
    let (mut a, note, base) = initial();
    let mut b = a.clone();
    let left = edit(&mut a, note, "note.md", Some("left"));
    let right = edit(&mut b, note, "note.md", Some("right"));
    a.import(b.revisions.values().cloned()).unwrap();
    let resolution = resolve(
        &a,
        left,
        right,
        Uuid::new_v4(),
        p("note.md"),
        Some(h("merged")),
    )
    .unwrap();
    let newer = edit(&mut a, note, "note.md", Some("newest"));
    let before = a.clone();
    assert_eq!(a.commit(resolution, Some(left)), Err(Error::Stale));
    assert_eq!(a.advance(note, Some(base), newer), Err(Error::Stale));
    assert_eq!(a, before);
}
#[test]
fn destination_collisions_refuse_both_create_and_rename() {
    let (mut j, note, _) = initial();
    let second = NoteId::new();
    let r = Revision::new(
        second,
        BTreeSet::new(),
        Uuid::new_v4(),
        p("other.md"),
        Some(h("other")),
    );
    j.commit(r, None).unwrap();
    let before = j.clone();
    let base = j.heads[&note];
    let moved = Revision::new(
        note,
        BTreeSet::from([base]),
        Uuid::new_v4(),
        p("other.md"),
        Some(h("base")),
    );
    assert_eq!(j.commit(moved, Some(base)), Err(Error::Collision));
    assert_eq!(j, before);
    let extra = Revision::new(
        NoteId::new(),
        BTreeSet::new(),
        Uuid::new_v4(),
        p("other.md"),
        Some(h("new")),
    );
    assert_eq!(j.commit(extra, None), Err(Error::Collision));
    assert_eq!(j, before);
}
#[test]
fn graph_rejects_cycles_orphans_forged_ids_and_foreign_note_parents_atomically() {
    let (mut j, note, base) = initial();
    let before = j.clone();
    let mut forged = j.revisions[&base].clone();
    forged.content = Some(h("forged"));
    assert_eq!(j.import([forged]), Err(Error::InvalidGraph));
    assert_eq!(j, before);
    let unknown = Revision::new(
        note,
        BTreeSet::from([Uuid::new_v4()]),
        Uuid::new_v4(),
        p("note.md"),
        Some(h("x")),
    );
    assert_eq!(j.import([unknown]), Err(Error::InvalidGraph));
    let foreign = Revision::new(
        NoteId::new(),
        BTreeSet::from([base]),
        Uuid::new_v4(),
        p("other.md"),
        Some(h("x")),
    );
    assert_eq!(j.import([foreign]), Err(Error::InvalidGraph));
    let mut cycle = before.clone();
    cycle.revisions.get_mut(&base).unwrap().parents.insert(base);
    assert_eq!(cycle.validate(), Err(Error::InvalidGraph));
    assert_eq!(j, before);
}
#[test]
fn unrelated_roots_cannot_claim_the_same_note_identity() {
    let (mut j, note, _) = initial();
    let other = Revision::new(
        note,
        BTreeSet::new(),
        Uuid::new_v4(),
        p("note.md"),
        Some(h("other")),
    );
    assert_eq!(j.import([other]), Err(Error::InvalidGraph));
}
#[test]
fn receipts_are_monotonic_and_tombstones_remain_after_acknowledgment() {
    let (mut j, note, base) = initial();
    let device = Uuid::new_v4();
    j.acknowledge(device, BTreeMap::from([(note, base)]))
        .unwrap();
    let deleted = edit(&mut j, note, "note.md", None);
    j.acknowledge(device, BTreeMap::from([(note, deleted)]))
        .unwrap();
    assert_eq!(
        j.acknowledge(device, BTreeMap::from([(note, base)])),
        Err(Error::Stale)
    );
    assert!(j.revisions.contains_key(&base));
    assert!(j.revisions.contains_key(&deleted));
}
#[test]
fn pairing_never_treats_populated_folders_as_empty_targets() {
    let local = File {
        note: NoteId::new(),
        path: p("a.md"),
        content: h("left"),
    };
    let remote = File {
        note: NoteId::new(),
        path: p("a.md"),
        content: h("right"),
    };
    assert_eq!(
        pair(
            PairingMode::Upload,
            std::slice::from_ref(&local),
            std::slice::from_ref(&remote)
        ),
        Err(Error::Pairing)
    );
    assert_eq!(
        pair(
            PairingMode::Download,
            std::slice::from_ref(&local),
            std::slice::from_ref(&remote)
        ),
        Err(Error::Pairing)
    );
    assert_eq!(
        pair(
            PairingMode::Reconcile,
            std::slice::from_ref(&local),
            std::slice::from_ref(&remote)
        )
        .unwrap(),
        vec![PairingAction::Conflict {
            local: local.clone(),
            remote: remote.clone()
        }]
    );
    let same = File {
        content: local.content.clone(),
        ..remote.clone()
    };
    assert!(matches!(
        &pair(PairingMode::Reconcile, &[local], &[same]).unwrap()[0],
        PairingAction::Link { .. }
    ));
}
#[test]
fn different_workspaces_require_explicit_pairing() {
    let (a, _, _) = initial();
    let mut b = a.clone();
    b.workspace = Uuid::new_v4();
    assert_eq!(plan(&a, &b), Err(Error::Pairing));
}
#[test]
fn future_schema_is_rejected_before_planning() {
    let (mut a, _, _) = initial();
    let b = a.clone();
    a.schema = 999;
    assert_eq!(plan(&a, &b), Err(Error::Schema));
}

#[test]
fn unrelated_notes_at_the_same_destination_produce_no_overwriting_action() {
    let (a, note, _) = initial();
    let mut b = Journal::new(a.workspace);
    let other = NoteId::new();
    b.commit(
        Revision::new(
            other,
            BTreeSet::new(),
            Uuid::new_v4(),
            p("note.md"),
            Some(h("other")),
        ),
        None,
    )
    .unwrap();
    assert_eq!(
        plan(&a, &b).unwrap(),
        vec![Action::PathCollision {
            path: p("note.md"),
            local: note,
            remote: other
        }]
    );
}
#[test]
fn touch_only_information_cannot_change_a_sync_plan() {
    let (a, _, _) = initial();
    let b = a.clone();
    // The transport carries no modified_at field to accidentally compare.
    let value = serde_json::to_value(&a).unwrap();
    assert!(!value.to_string().contains("mtime"));
    assert!(plan(&a, &b).unwrap().is_empty());
}
