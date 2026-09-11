pub mod control;
pub mod remote;
pub mod state;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("application blocked; close the workspace and preserve local changes or drafts before retrying")]
    ApplicationBlocked,
    #[error("this application step does not yet support renames or deletions; received content was retained")]
    UnsupportedApplication,
    #[error("invalid client configuration or state")]
    Invalid,
    #[error("client state cannot be read or saved; preserve it for recovery")]
    Storage,
    #[error("client or server is busy; retained revisions can be retried")]
    Busy,
    #[error("offline or TLS connection failed; pending revisions were retained")]
    Offline,
    #[error("remote access was denied; check the credential and scope")]
    Denied,
    #[error("remote revision conflicts with this queue; pending revisions were retained")]
    Conflict,
    #[error("capacity reached; existing revisions were retained")]
    Limit,
    #[error("remote response was rejected; pending revisions were retained")]
    Protocol,
}
pub type Result<T> = std::result::Result<T, Error>;
