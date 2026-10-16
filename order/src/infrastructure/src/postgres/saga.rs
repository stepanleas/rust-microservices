use crate::DbPool;
use crate::postgres::order::{insert_order, update_order_status};
use crate::schema::order_sagas;
use application::outbox::OrderOutboxMessage;
use application::ports::out::saga_store::OrderSagaRepository;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use domain::aggregates::order::Order;
use domain::saga::{OrderSaga, SagaStatus};
use shared::domain::value_objects::OrderId;
use shared::outbox::postgres::insert_outbox_message;
use uuid::Uuid;

#[derive(Queryable, Selectable, Insertable, PartialEq, Debug)]
#[diesel(table_name = order_sagas)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct OrderSagaRow {
    id: Uuid,
    order_id: Uuid,
    status: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl From<&OrderSaga> for OrderSagaRow {
    fn from(saga: &OrderSaga) -> Self {
        Self {
            id: saga.id(),
            order_id: saga.order_id().into(),
            status: saga.status().as_str().to_string(),
            created_at: saga.started_at().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl TryFrom<OrderSagaRow> for OrderSaga {
    type Error = anyhow::Error;

    fn try_from(row: OrderSagaRow) -> Result<Self, Self::Error> {
        Ok(OrderSaga::restore(
            row.id,
            OrderId::from_uuid(row.order_id),
            SagaStatus::try_from(row.status.as_str())?,
            row.created_at.and_utc(),
        ))
    }
}

pub struct PostgresOrderSagaRepository {
    pool: DbPool,
}

impl PostgresOrderSagaRepository {
    pub fn new(pool: &DbPool) -> Self {
        Self { pool: pool.clone() }
    }
}

impl OrderSagaRepository for PostgresOrderSagaRepository {
    fn find_by_id(&self, saga_id: Uuid) -> anyhow::Result<Option<OrderSaga>> {
        let mut connection = self.pool.get()?;

        order_sagas::table
            .find(saga_id)
            .select(OrderSagaRow::as_select())
            .first(&mut connection)
            .optional()?
            .map(OrderSaga::try_from)
            .transpose()
    }

    fn start(
        &self,
        order: &Order,
        saga: &OrderSaga,
        message: &OrderOutboxMessage,
    ) -> anyhow::Result<()> {
        let mut connection = self.pool.get()?;

        connection.transaction::<_, anyhow::Error, _>(|conn| {
            insert_order(conn, order)?;

            diesel::insert_into(order_sagas::table)
                .values(&OrderSagaRow::from(saga))
                .execute(conn)?;

            insert_outbox_message(conn, message)?;

            Ok(())
        })
    }

    fn advance(
        &self,
        order: &Order,
        saga: &OrderSaga,
        expected_status: SagaStatus,
        message: Option<OrderOutboxMessage>,
    ) -> anyhow::Result<bool> {
        let mut connection = self.pool.get()?;

        connection.transaction::<_, anyhow::Error, _>(|conn| {
            // Optimistic check: only move the saga if nobody else moved it in the meantime.
            let updated = diesel::update(
                order_sagas::table
                    .filter(order_sagas::id.eq(saga.id()))
                    .filter(order_sagas::status.eq(expected_status.as_str())),
            )
            .set((
                order_sagas::status.eq(saga.status().as_str()),
                order_sagas::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn)?;

            if updated == 0 {
                return Ok(false);
            }

            update_order_status(conn, order)?;

            if let Some(message) = &message {
                insert_outbox_message(conn, message)?;
            }

            Ok(true)
        })
    }
}
