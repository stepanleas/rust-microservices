use crate::DbPool;
use crate::schema::payments;
use application::outbox::PaymentOutboxMessage;
use application::ports::out::repositories::PaymentRepository;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use domain::aggregates::payment::{Payment, PaymentId};
use domain::enums::PaymentStatus;
use rust_decimal::Decimal;
use shared::domain::value_objects::{CustomerId, OrderId};
use shared::outbox::postgres::insert_outbox_message;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Queryable, Selectable, Identifiable, Insertable, AsChangeset, PartialEq, Debug)]
#[diesel(table_name = payments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct PaymentRow {
    id: Uuid,
    order_id: Uuid,
    customer_id: Uuid,
    price: BigDecimal,
    payment_status: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    failure_messages: Vec<String>,
}

impl From<Payment> for PaymentRow {
    fn from(payment: Payment) -> Self {
        Self {
            id: payment.id().into(),
            order_id: payment.order_id().into(),
            customer_id: payment.customer_id().into(),
            price: BigDecimal::from_str(&payment.price().to_string())
                .unwrap_or(BigDecimal::from(0)),
            payment_status: payment.payment_status().as_str().to_string(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            failure_messages: payment.failure_messages().to_vec(),
        }
    }
}

impl From<PaymentRow> for Payment {
    fn from(row: PaymentRow) -> Self {
        Payment::builder()
            .id(PaymentId::from_uuid(row.id))
            .order_id(OrderId::from_uuid(row.order_id))
            .customer_id(CustomerId::from_uuid(row.customer_id))
            .price(Decimal::from_str(&row.price.to_string()).unwrap_or(Decimal::ZERO))
            .payment_status(PaymentStatus::parse(&row.payment_status).unwrap_or_default())
            .failure_messages(row.failure_messages)
            .build()
    }
}

pub struct PostgresPaymentRepository {
    pool: DbPool,
}

impl PostgresPaymentRepository {
    pub fn new(pool: &DbPool) -> Self {
        Self { pool: pool.clone() }
    }
}

impl PaymentRepository for PostgresPaymentRepository {
    fn save(&self, payment: Payment) -> anyhow::Result<Payment> {
        let mut connection = self.pool.get()?;

        let row = PaymentRow::from(payment);

        diesel::insert_into(payments::table)
            .values(&row)
            .on_conflict(payments::id)
            .do_update()
            .set(&row)
            .execute(&mut connection)?;

        Ok(row.into())
    }

    fn save_with_outbox(
        &self,
        payment: Payment,
        message: PaymentOutboxMessage,
    ) -> anyhow::Result<Payment> {
        let mut connection = self.pool.get()?;

        let row = PaymentRow::from(payment);

        connection.transaction::<_, diesel::result::Error, _>(|conn| {
            diesel::insert_into(payments::table)
                .values(&row)
                .execute(conn)?;

            insert_outbox_message(conn, &message)
        })?;

        Ok(row.into())
    }

    fn find_by_id(&self, payment_id: PaymentId) -> anyhow::Result<Option<Payment>> {
        let mut connection = self.pool.get()?;

        let row = payments::table
            .find(payment_id.as_uuid())
            .select(PaymentRow::as_select())
            .first(&mut connection)
            .optional()?;

        Ok(row.map(Payment::from))
    }

    fn find_by_order_id(&self, order_id: OrderId) -> anyhow::Result<Option<Payment>> {
        let mut connection = self.pool.get()?;

        let row = payments::table
            .filter(payments::order_id.eq(order_id.as_uuid()))
            .select(PaymentRow::as_select())
            .first(&mut connection)
            .optional()?;

        Ok(row.map(Payment::from))
    }
}
