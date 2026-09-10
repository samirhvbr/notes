pub mod admin;
pub mod api;
pub mod backup;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
