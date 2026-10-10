pub mod postgres;
mod schema;

use diesel_migrations::{EmbeddedMigrations, embed_migrations};

pub use shared::db::DbPool;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./src/migrations");

fn to_big_decimal(value: rust_decimal::Decimal) -> bigdecimal::BigDecimal {
    use std::str::FromStr;
    bigdecimal::BigDecimal::from_str(&value.to_string()).unwrap_or_default()
}

fn to_decimal(value: &bigdecimal::BigDecimal) -> rust_decimal::Decimal {
    use std::str::FromStr;
    rust_decimal::Decimal::from_str(&value.to_string()).unwrap_or_default()
}
