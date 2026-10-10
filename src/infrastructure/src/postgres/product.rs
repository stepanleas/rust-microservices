use crate::schema::products;
use crate::{DbPool, to_big_decimal, to_decimal};
use application::ports::out::repositories::ProductRepository;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use domain::aggregates::product::Product;
use shared::domain::value_objects::ProductId;
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

impl From<&Product> for ProductRow {
    fn from(product: &Product) -> Self {
        Self {
            id: product.id().into(),
            title: product.title().into(),
            quantity: product.quantity(),
            price: to_big_decimal(product.price()),
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
            .price(to_decimal(&row.price))
            .build()
    }
}

/// Writes the stock of `updated` products with the caller's connection, so it joins the
/// caller's transaction.
pub(crate) fn update_stock(connection: &mut PgConnection, updated: &[Product]) -> QueryResult<()> {
    for product in updated {
        diesel::update(products::table.find(product.id().as_uuid()))
            .set((
                products::quantity.eq(product.quantity()),
                products::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(connection)?;
    }

    Ok(())
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
    fn find_all(&self) -> anyhow::Result<Vec<Product>> {
        let mut connection = self.pool.get()?;

        let rows: Vec<ProductRow> = products::table
            .order(products::title.asc())
            .select(ProductRow::as_select())
            .load(&mut connection)?;

        Ok(rows.into_iter().map(Product::from).collect())
    }

    fn find_by_id(&self, product_id: ProductId) -> anyhow::Result<Option<Product>> {
        let mut connection = self.pool.get()?;

        Ok(products::table
            .find(product_id.as_uuid())
            .select(ProductRow::as_select())
            .first(&mut connection)
            .optional()?
            .map(Product::from))
    }

    fn find_by_ids(&self, product_ids: Vec<ProductId>) -> anyhow::Result<Vec<Product>> {
        let mut connection = self.pool.get()?;

        let rows: Vec<ProductRow> = products::table
            .filter(products::id.eq_any(product_ids.iter().map(ProductId::as_uuid)))
            .select(ProductRow::as_select())
            .load(&mut connection)?;

        Ok(rows.into_iter().map(Product::from).collect())
    }

    fn save(&self, product: &Product) -> anyhow::Result<()> {
        let mut connection = self.pool.get()?;

        let row = ProductRow::from(product);

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
