use crate::DbPool;
use crate::schema::customers::dsl::customers;
use crate::schema::customers::id;
use application::ports::out::repositories::CustomerRepository;
use chrono::NaiveDateTime;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, RunQueryDsl, Selectable};
use domain::aggregates::customer::Customer;
use shared::domain::value_objects::CustomerId;
use uuid::Uuid;

#[derive(Queryable, Selectable, Identifiable, Insertable, AsChangeset, PartialEq, Debug)]
#[diesel(table_name = crate::schema::customers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CustomerEntity {
    id: Uuid,
    user_name: String,
    first_name: String,
    last_name: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl From<Customer> for CustomerEntity {
    fn from(customer: Customer) -> Self {
        Self {
            id: customer.id().into(),
            user_name: customer.user_name().into(),
            first_name: customer.first_name().into(),
            last_name: customer.last_name().into(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl From<CustomerEntity> for Customer {
    fn from(entity: CustomerEntity) -> Self {
        Customer::builder()
            .id(CustomerId::from_uuid(entity.id))
            .user_name(entity.user_name)
            .first_name(entity.first_name)
            .last_name(entity.last_name)
            .build()
    }
}

pub struct PostgresCustomerRepository {
    pool: DbPool,
}

impl PostgresCustomerRepository {
    pub fn new(pool: &DbPool) -> Self {
        Self { pool: pool.clone() }
    }
}

impl CustomerRepository for PostgresCustomerRepository {
    fn save(&self, entity: Customer) -> anyhow::Result<Customer> {
        let mut connection = self.pool.get()?;

        let persistent_entity = CustomerEntity::from(entity);

        diesel::insert_into(customers)
            .values(&persistent_entity)
            .on_conflict(id)
            .do_update()
            .set(&persistent_entity)
            .execute(&mut connection)?;

        Ok(persistent_entity.into())
    }
}
