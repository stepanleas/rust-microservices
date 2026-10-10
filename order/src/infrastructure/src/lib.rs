pub mod postgres;
mod schema;

use diesel_migrations::{EmbeddedMigrations, embed_migrations};

pub use shared::db::DbPool;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./src/migrations");
