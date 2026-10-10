use crate::DbPool;
use crate::schema::customers;
use application::ports::out::repositories::CustomerRepository;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use domain::aggregates::customer::Customer;
use shared::domain::value_objects::CustomerId;
use uuid::Uuid;

#[derive(Queryable, Selectable, Identifiable, Insertable, AsChangeset, PartialEq, Debug)]
#[diesel(table_name = customers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct CustomerRow {
    id: Uuid,
    user_name: String,
    first_name: String,
    last_name: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl From<Customer> for CustomerRow {
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

impl From<CustomerRow> for Customer {
    fn from(row: CustomerRow) -> Self {
        Customer::builder()
            .id(CustomerId::from_uuid(row.id))
            .user_name(row.user_name)
            .first_name(row.first_name)
            .last_name(row.last_name)
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
    fn find_by_id(&self, customer_id: CustomerId) -> anyhow::Result<Option<Customer>> {
        let mut connection = self.pool.get()?;

        let row = customers::table
            .find(customer_id.as_uuid())
            .select(CustomerRow::as_select())
            .first(&mut connection)
            .optional()?;

        Ok(row.map(Customer::from))
    }

    fn save(&self, entity: Customer) -> anyhow::Result<()> {
        let mut connection = self.pool.get()?;

        let row = CustomerRow::from(entity);

        diesel::insert_into(customers::table)
            .values(&row)
            .on_conflict(customers::id)
            .do_update()
            .set(&row)
            .execute(&mut connection)?;

        Ok(())
    }
}
