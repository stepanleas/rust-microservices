use crate::commands::{CreateOrderCommand, CreateOrderItemDto};
use domain::aggregates::order::{Order, OrderItem};
use rusty_money::{Money, iso};
use shared::domain::value_objects::{CustomerId, OrderId, OrderItemId, ProductId};

pub struct OrderMapper;

impl OrderMapper {
    pub fn map_create_order_command_to_domain_entity(command: CreateOrderCommand) -> Order {
        let mut order = Order::builder()
            .id(OrderId::new())
            .customer_id(CustomerId::from_uuid(command.customer_id()))
            .price(Money::from_decimal(command.price(), iso::USD))
            .build();

        let order_items =
            Self::map_create_order_item_dto_to_domain_entity(command.items(), order.id());
        order.set_items(order_items);

        order
    }

    fn map_create_order_item_dto_to_domain_entity(
        items: &[CreateOrderItemDto],
        order_id: OrderId,
    ) -> Vec<OrderItem> {
        items
            .iter()
            .map(|item| {
                OrderItem::builder()
                    .id(OrderItemId::new())
                    .order_id(order_id)
                    .product_id(ProductId::from_uuid(item.product_id()))
                    .quantity(item.quantity())
                    .price(Money::from_decimal(item.price(), iso::USD))
                    .sub_total(Money::from_decimal(item.sub_total(), iso::USD))
                    .build()
            })
            .collect()
    }
}
