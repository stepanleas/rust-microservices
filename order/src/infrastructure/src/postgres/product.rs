use crate::DbPool;
use crate::schema::products;
use application::ports::out::repositories::ProductRepository;
use chrono::NaiveDateTime;
use diesel::internal::derives::multiconnection::bigdecimal::{BigDecimal, Zero};
use diesel::prelude::*;
use domain::aggregates::product::Product;
use rusty_money::{Money, iso};
use shared::domain::value_objects::ProductId;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Queryable, Selectable, Identifiable, Insertable, AsChangeset, PartialEq, Debug)]
#[diesel(table_name = products)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct ProductRow {
    id: Uuid,
    title: String,
    quantity: i32,
    price: BigDecimal,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl From<Product> for ProductRow {
    fn from(product: Product) -> Self {
        Self {
            id: product.id().into(),
            title: product.title().into(),
            quantity: product.quantity(),
            price: BigDecimal::from_str(product.price().amount().to_string().as_str())
                .unwrap_or(BigDecimal::zero()),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<ProductRow> for Product {
    fn from(row: ProductRow) -> Self {
        Product::builder()
            .id(ProductId::from_uuid(row.id))
            .title(row.title)
            .quantity(row.quantity)
            .price(
                Money::from_str(row.price.to_string().as_str(), iso::USD)
                    .unwrap_or(Money::from_minor(0, iso::USD)),
            )
            .build()
    }
}

pub struct PostgresProductRepository {
    pool: DbPool,
}

impl PostgresProductRepository {
    pub fn new(pool: &DbPool) -> Self {
        Self { pool: pool.clone() }
    }
}

impl ProductRepository for PostgresProductRepository {
    fn find_by_ids(&self, product_ids: Vec<ProductId>) -> anyhow::Result<Vec<Product>> {
        let mut connection = self.pool.get()?;

        let rows: Vec<ProductRow> = products::table
            .filter(products::id.eq_any(product_ids.iter().map(|product_id| product_id.as_uuid())))
            .select(ProductRow::as_select())
            .load(&mut connection)?;

        Ok(rows.into_iter().map(Product::from).collect())
    }

    fn find_by_id(&self, product_id: ProductId) -> anyhow::Result<Option<Product>> {
        let mut connection = self.pool.get()?;

        let row = products::table
            .find(product_id.as_uuid())
            .select(ProductRow::as_select())
            .first(&mut connection)
            .optional()?;

        Ok(row.map(Product::from))
    }

    fn save(&self, entity: Product) -> anyhow::Result<()> {
        let mut connection = self.pool.get()?;

        let row = ProductRow::from(entity);

        diesel::insert_into(products::table)
            .values(&row)
            .on_conflict(products::id)
            .do_update()
            .set(&row)
            .execute(&mut connection)?;

        Ok(())
    }

    fn delete(&self, product_id: ProductId) -> anyhow::Result<()> {
        let mut connection = self.pool.get()?;

        diesel::delete(products::table.find(product_id.as_uuid())).execute(&mut connection)?;

        Ok(())
    }
}
