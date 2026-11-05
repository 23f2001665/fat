pub mod database;
pub mod error;
pub mod models;
pub mod services;
pub mod ui;
pub mod updates;
pub mod utils;

pub use database::Database;
pub use error::{AppError, AppResult};
