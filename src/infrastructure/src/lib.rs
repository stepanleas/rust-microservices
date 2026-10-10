use diesel_migrations::{EmbeddedMigrations, embed_migrations};

pub mod postgres;
pub mod schema;

pub use shared::db::DbPool;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./src/migrations");
