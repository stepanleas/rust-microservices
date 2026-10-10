use crate::DbPool;
use crate::schema::categories::dsl::categories;
use crate::schema::categories::id;
use application::ports::out::repositories::CategoryRepository;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use domain::aggregates::category::Category;
use shared::domain::value_objects::CategoryId;
use uuid::Uuid;

#[derive(Queryable, Selectable, Identifiable, Insertable, AsChangeset, PartialEq, Debug)]
#[diesel(table_name = crate::schema::categories)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct CategoryRow {
    id: Uuid,
    title: String,
    description: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl From<Category> for CategoryRow {
    fn from(category: Category) -> Self {
        Self {
            id: category.id().into(),
            title: category.title().into(),
            description: category.description().into(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<CategoryRow> for Category {
    fn from(row: CategoryRow) -> Self {
        Category::new(CategoryId::from_uuid(row.id), row.title, row.description)
    }
}

pub struct PostgresCategoryRepository {
    pool: DbPool,
}

impl PostgresCategoryRepository {
    pub fn new(pool: &DbPool) -> Self {
        Self { pool: pool.clone() }
    }
}

impl CategoryRepository for PostgresCategoryRepository {
    fn list_all(&self) -> anyhow::Result<Vec<Category>> {
        let mut connection = self.pool.get()?;
        let rows: Vec<CategoryRow> = categories.load(&mut connection)?;

        Ok(rows.into_iter().map(Category::from).collect())
    }

    fn find_by_id(&self, entity_id: CategoryId) -> anyhow::Result<Option<Category>> {
        let mut connection = self.pool.get()?;

        let row = categories
            .filter(id.eq(entity_id.as_uuid()))
            .first::<CategoryRow>(&mut connection)
            .optional()?;

        Ok(row.map(Category::from))
    }

    fn save(&self, entity: Category) -> anyhow::Result<Category> {
        let mut connection = self.pool.get()?;

        let row = CategoryRow::from(entity);

        diesel::insert_into(categories)
            .values(&row)
            .on_conflict(id)
            .do_update()
            .set(&row)
            .execute(&mut connection)?;

        Ok(row.into())
    }

    fn delete(&self, entity_id: CategoryId) -> anyhow::Result<()> {
        let mut connection = self.pool.get()?;
        diesel::delete(categories.filter(id.eq(entity_id.as_uuid()))).execute(&mut connection)?;

        Ok(())
    }
}
