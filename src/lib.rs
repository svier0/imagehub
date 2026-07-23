pub mod api;
pub mod config;
pub mod errors;
pub mod models;
pub mod services;

pub use errors::Error;
pub use models::ImageInfo;

pub type Result<T> = std::result::Result<T, Error>;
