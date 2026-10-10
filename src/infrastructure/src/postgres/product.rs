use crate::DbPool;
use crate::postgres::category::CategoryRow;
use crate::schema::categories::dsl::categories;
use crate::schema::products::dsl::products;
use application::ports::out::repositories::ProductRepository;
use chrono::NaiveDateTime;
use diesel::internal::derives::multiconnection::bigdecimal::BigDecimal;
use diesel::{
    AsChangeset, Associations, ExpressionMethods, Identifiable, Insertable, OptionalExtension,
    QueryDsl, Queryable, RunQueryDsl, Selectable, SelectableHelper,
};
use domain::aggregates::product::Product;
use rusty_money::{Money, iso};
use shared::domain::value_objects::ProductId;
use std::str::FromStr;
use uuid::Uuid;

#[derive(
    Queryable, Selectable, Identifiable, Associations, Insertable, AsChangeset, PartialEq, Debug,
)]
#[diesel(belongs_to(CategoryRow, foreign_key = category_id))]
#[diesel(table_name = crate::schema::products)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct ProductRow {
    id: Uuid,
    category_id: Uuid,
    title: String,
    description: String,
    quantity: i32,
    price: BigDecimal,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl ProductRow {
    fn into_domain(self, category_row: CategoryRow) -> Product {
        let price = Money::from_str(self.price.to_string().as_str(), iso::USD)
            .unwrap_or(Money::from_minor(0, iso::USD));

        Product::builder()
            .id(ProductId::from_uuid(self.id))
            .title(&self.title)
            .description(&self.description)
            .quantity(self.quantity)
            .price(price)
            .category(category_row.into())
            .build()
    }
}

impl From<Product> for ProductRow {
    fn from(product: Product) -> Self {
        let price = BigDecimal::from_str(product.price().amount().to_string().as_str())
            .unwrap_or(BigDecimal::from(0));

        Self {
            id: product.id().into(),
            category_id: product.category().id().into(),
            title: product.title().into(),
            description: product.description().into(),
            quantity: product.quantity(),
            price,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
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
    fn list_all(&self) -> anyhow::Result<Vec<Product>> {
        let mut connection = self.pool.get()?;

        let rows: Vec<(ProductRow, CategoryRow)> = products
            .inner_join(categories)
            .select((ProductRow::as_select(), CategoryRow::as_select()))
            .load::<(ProductRow, CategoryRow)>(&mut connection)?;

        Ok(rows
            .into_iter()
            .map(|(product_row, category_row)| product_row.into_domain(category_row))
            .collect())
    }

    fn find_by_id(&self, entity_id: ProductId) -> anyhow::Result<Option<Product>> {
        let mut connection = self.pool.get()?;

        let row = products
            .inner_join(categories)
            .filter(crate::schema::products::id.eq(entity_id.as_uuid()))
            .select((ProductRow::as_select(), CategoryRow::as_select()))
            .first::<(ProductRow, CategoryRow)>(&mut connection)
            .optional()?;

        Ok(row.map(|(product_row, category_row)| product_row.into_domain(category_row)))
    }

    fn save(&self, entity: Product) -> anyhow::Result<Product> {
        let mut connection = self.pool.get()?;

        let row = ProductRow::from(entity);
        let product_row = diesel::insert_into(products)
            .values(&row)
            .on_conflict(crate::schema::products::id)
            .do_update()
            .set(&row)
            .returning(ProductRow::as_returning())
            .get_result::<ProductRow>(&mut connection)?;

        let category_row = categories
            .find(product_row.category_id)
            .first::<CategoryRow>(&mut connection)?;

        Ok(product_row.into_domain(category_row))
    }

    fn delete(&self, entity_id: ProductId) -> anyhow::Result<()> {
        let mut connection = self.pool.get()?;
        diesel::delete(products.filter(crate::schema::products::id.eq(entity_id.as_uuid())))
            .execute(&mut connection)?;

        Ok(())
    }
}
