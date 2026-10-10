use crate::DbPool;
use crate::schema::{order_items, orders};
use application::ports::out::repositories::OrderRepository;
use chrono::NaiveDateTime;
use diesel::deserialize::{self, FromSql};
use diesel::internal::derives::multiconnection::bigdecimal::{BigDecimal, Zero};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::VarChar;
use diesel::{AsExpression, FromSqlRow};
use domain::aggregates::order::{Order, OrderItem};
use domain::enums::OrderStatus as DomainOrderStatus;
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CustomerId, OrderId, OrderItemId, ProductId};
use std::io::Write;
use std::str::FromStr;
use uuid::Uuid;

#[derive(AsExpression, FromSqlRow, PartialEq, Debug, Clone, Copy)]
#[diesel(sql_type = VarChar)]
enum OrderStatus {
    Pending,
    Paid,
    Approved,
    Cancelling,
    Canceled,
}

impl From<OrderStatus> for DomainOrderStatus {
    fn from(value: OrderStatus) -> Self {
        match value {
            OrderStatus::Pending => DomainOrderStatus::Pending,
            OrderStatus::Paid => DomainOrderStatus::Paid,
            OrderStatus::Approved => DomainOrderStatus::Approved,
            OrderStatus::Cancelling => DomainOrderStatus::Cancelling,
            OrderStatus::Canceled => DomainOrderStatus::Canceled,
        }
    }
}

impl From<DomainOrderStatus> for OrderStatus {
    fn from(value: DomainOrderStatus) -> Self {
        match value {
            DomainOrderStatus::Pending => OrderStatus::Pending,
            DomainOrderStatus::Paid => OrderStatus::Paid,
            DomainOrderStatus::Approved => OrderStatus::Approved,
            DomainOrderStatus::Cancelling => OrderStatus::Cancelling,
            DomainOrderStatus::Canceled => OrderStatus::Canceled,
        }
    }
}

impl ToSql<VarChar, Pg> for OrderStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let status_str = match self {
            OrderStatus::Pending => "pending",
            OrderStatus::Paid => "paid",
            OrderStatus::Approved => "approved",
            OrderStatus::Cancelling => "cancelling",
            OrderStatus::Canceled => "cancelled",
        };
        out.write_all(status_str.as_bytes())?;
        Ok(IsNull::No)
    }
}

impl FromSql<VarChar, Pg> for OrderStatus {
    fn from_sql(bytes: diesel::pg::PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"pending" => Ok(OrderStatus::Pending),
            b"paid" => Ok(OrderStatus::Paid),
            b"approved" => Ok(OrderStatus::Approved),
            b"cancelling" => Ok(OrderStatus::Cancelling),
            b"cancelled" => Ok(OrderStatus::Canceled),
            v => Err(format!("Unknown order status: {:?}", String::from_utf8_lossy(v)).into()),
        }
    }
}

fn to_money(amount: &BigDecimal) -> Money<'static, iso::Currency> {
    Money::from_str(amount.to_string().as_str(), iso::USD).unwrap_or(Money::from_minor(0, iso::USD))
}

fn to_big_decimal(money: &Money<'static, iso::Currency>) -> BigDecimal {
    BigDecimal::from_str(money.amount().to_string().as_str()).unwrap_or(BigDecimal::zero())
}

#[derive(Queryable, Selectable, Identifiable, Insertable, AsChangeset, PartialEq, Debug)]
#[diesel(table_name = orders)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct OrderRow {
    id: Uuid,
    customer_id: Uuid,
    tracking_id: Uuid,
    price: BigDecimal,
    status: OrderStatus,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    failure_messages: Vec<String>,
}

impl OrderRow {
    fn into_order(self, items: Vec<OrderItemRow>) -> Order {
        Order::builder()
            .id(OrderId::from_uuid(self.id))
            .customer_id(CustomerId::from_uuid(self.customer_id))
            .tracking_id(self.tracking_id)
            .price(to_money(&self.price))
            .items(items.into_iter().map(OrderItem::from).collect())
            .status(self.status.into())
            .failure_messages(self.failure_messages)
            .build()
    }
}

impl From<&Order> for OrderRow {
    fn from(order: &Order) -> Self {
        Self {
            id: order.id().into(),
            customer_id: order.customer_id().into(),
            tracking_id: order.tracking_id(),
            price: to_big_decimal(order.price()),
            status: order.status().into(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            failure_messages: order.failure_messages().to_vec(),
        }
    }
}

#[derive(
    Queryable, Selectable, Identifiable, Associations, Insertable, AsChangeset, PartialEq, Debug,
)]
#[diesel(belongs_to(OrderRow, foreign_key = order_id))]
#[diesel(table_name = order_items)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct OrderItemRow {
    id: Uuid,
    order_id: Uuid,
    product_id: Uuid,
    quantity: i32,
    price: BigDecimal,
    sub_total: BigDecimal,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl From<OrderItemRow> for OrderItem {
    fn from(row: OrderItemRow) -> Self {
        OrderItem::builder()
            .id(OrderItemId::from_uuid(row.id))
            .order_id(OrderId::from_uuid(row.order_id))
            .product_id(ProductId::from_uuid(row.product_id))
            .quantity(row.quantity)
            .price(to_money(&row.price))
            .sub_total(to_money(&row.sub_total))
            .build()
    }
}

impl From<&OrderItem> for OrderItemRow {
    fn from(item: &OrderItem) -> Self {
        Self {
            id: item.id().into(),
            order_id: item.order_id().into(),
            product_id: item.product_id().into(),
            quantity: item.quantity(),
            price: to_big_decimal(item.price()),
            sub_total: to_big_decimal(item.sub_total()),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

/// Inserts the order and its items using the given connection (callers own the transaction).
pub(crate) fn insert_order(connection: &mut PgConnection, order: &Order) -> QueryResult<()> {
    diesel::insert_into(orders::table)
        .values(&OrderRow::from(order))
        .execute(connection)?;

    let items: Vec<OrderItemRow> = order.items().iter().map(OrderItemRow::from).collect();

    diesel::insert_into(order_items::table)
        .values(&items)
        .execute(connection)?;

    Ok(())
}

/// Persists the lifecycle fields of an existing order (status and failure messages).
pub(crate) fn update_order_status(connection: &mut PgConnection, order: &Order) -> QueryResult<()> {
    let updated = diesel::update(orders::table.find(order.id().as_uuid()))
        .set((
            orders::status.eq(OrderStatus::from(order.status())),
            orders::failure_messages.eq(order.failure_messages().to_vec()),
            orders::updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(connection)?;

    if updated == 0 {
        return Err(diesel::result::Error::NotFound);
    }

    Ok(())
}

pub struct PostgresOrderRepository {
    pool: DbPool,
}

impl PostgresOrderRepository {
    pub fn new(pool: &DbPool) -> Self {
        Self { pool: pool.clone() }
    }

    fn with_items(connection: &mut PgConnection, row: OrderRow) -> anyhow::Result<Order> {
        let items = OrderItemRow::belonging_to(&row)
            .select(OrderItemRow::as_select())
            .order(order_items::created_at.asc())
            .load(connection)?;

        Ok(row.into_order(items))
    }

    fn find_one<F>(&self, filter: F) -> anyhow::Result<Option<Order>>
    where
        F: FnOnce(&mut PgConnection) -> QueryResult<Option<OrderRow>>,
    {
        let mut connection = self.pool.get()?;

        match filter(&mut connection)? {
            Some(row) => Ok(Some(Self::with_items(&mut connection, row)?)),
            None => Ok(None),
        }
    }
}

impl OrderRepository for PostgresOrderRepository {
    fn find_by_id(&self, order_id: OrderId) -> anyhow::Result<Option<Order>> {
        self.find_one(|connection| {
            orders::table
                .find(order_id.as_uuid())
                .select(OrderRow::as_select())
                .first(connection)
                .optional()
        })
    }

    fn find_by_tracking_id(&self, tracking_id: Uuid) -> anyhow::Result<Option<Order>> {
        self.find_one(|connection| {
            orders::table
                .filter(orders::tracking_id.eq(tracking_id))
                .select(OrderRow::as_select())
                .first(connection)
                .optional()
        })
    }

    fn save(&self, entity: &Order) -> anyhow::Result<()> {
        let mut connection = self.pool.get()?;

        connection.transaction(|conn| insert_order(conn, entity))?;

        Ok(())
    }
}
