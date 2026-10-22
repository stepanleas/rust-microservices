use crate::DbPool;
use crate::postgres::product::apply_stock_changes;
use crate::schema::order_approvals;
use application::outbox::RestaurantOutboxMessage;
use application::ports::out::repositories::{OrderApprovalRepository, StockChange};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use domain::aggregates::order_approval::OrderApproval;
use domain::enums::OrderApprovalStatus;
use domain::value_objects::OrderApprovalId;
use shared::domain::value_objects::OrderId;
use shared::outbox::postgres::insert_outbox_message;
use uuid::Uuid;

#[derive(Queryable, Selectable, Identifiable, Insertable, PartialEq, Debug)]
#[diesel(table_name = order_approvals)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct OrderApprovalRow {
    id: Uuid,
    saga_id: Uuid,
    order_id: Uuid,
    status: String,
    failure_messages: Vec<String>,
    created_at: NaiveDateTime,
}

impl From<&OrderApproval> for OrderApprovalRow {
    fn from(approval: &OrderApproval) -> Self {
        Self {
            id: approval.id().into(),
            saga_id: approval.saga_id(),
            order_id: approval.order_id().into(),
            status: approval.status().as_str().to_string(),
            failure_messages: approval.failure_messages().to_vec(),
            created_at: approval.created_at().naive_utc(),
        }
    }
}

impl TryFrom<OrderApprovalRow> for OrderApproval {
    type Error = anyhow::Error;

    fn try_from(row: OrderApprovalRow) -> Result<Self, Self::Error> {
        Ok(OrderApproval::new(
            OrderApprovalId::from_uuid(row.id),
            row.saga_id,
            OrderId::from_uuid(row.order_id),
            OrderApprovalStatus::try_from(row.status.as_str())?,
            row.failure_messages,
            row.created_at.and_utc(),
        ))
    }
}

pub struct PostgresOrderApprovalRepository {
    pool: DbPool,
}

impl PostgresOrderApprovalRepository {
    pub fn new(pool: &DbPool) -> Self {
        Self { pool: pool.clone() }
    }
}

impl OrderApprovalRepository for PostgresOrderApprovalRepository {
    fn find_by_order_id(&self, order_id: OrderId) -> anyhow::Result<Option<OrderApproval>> {
        let mut connection = self.pool.get()?;

        order_approvals::table
            .filter(order_approvals::order_id.eq(order_id.as_uuid()))
            .select(OrderApprovalRow::as_select())
            .first(&mut connection)
            .optional()?
            .map(OrderApproval::try_from)
            .transpose()
    }

    fn save_with_outbox(
        &self,
        approval: &OrderApproval,
        stock_changes: &[StockChange],
        message: &RestaurantOutboxMessage,
    ) -> anyhow::Result<()> {
        let mut connection = self.pool.get()?;

        connection.transaction::<(), anyhow::Error, _>(|conn| {
            diesel::insert_into(order_approvals::table)
                .values(OrderApprovalRow::from(approval))
                .execute(conn)?;

            apply_stock_changes(conn, stock_changes)?;
            insert_outbox_message(conn, message)?;

            Ok(())
        })
    }

    fn save_cancellation(
        &self,
        approval: &OrderApproval,
        stock_changes: &[StockChange],
    ) -> anyhow::Result<()> {
        let mut connection = self.pool.get()?;

        connection.transaction::<(), anyhow::Error, _>(|conn| {
            // Only an approval that is still APPROVED in the database may be cancelled, so a
            // concurrent duplicate cannot give the stock back twice.
            let cancelled = diesel::update(
                order_approvals::table
                    .find(approval.id().as_uuid())
                    .filter(order_approvals::status.eq(OrderApprovalStatus::Approved.as_str())),
            )
            .set(order_approvals::status.eq(approval.status().as_str()))
            .execute(conn)?;

            if cancelled == 0 {
                anyhow::bail!(
                    "Approval of order {} is no longer APPROVED, not cancelling it",
                    approval.order_id(),
                );
            }

            apply_stock_changes(conn, stock_changes)?;

            Ok(())
        })
    }
}
