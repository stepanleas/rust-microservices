//! The CPU side of POST /api/orders before any I/O: mapping the command to the domain order and
//! validating its prices (`cargo bench -p application --bench create_order`).
use std::hint::black_box;

use application::commands::{CreateOrderCommand, CreateOrderItemDto};
use application::mappers::OrderMapper;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rust_decimal::Decimal;
use uuid::Uuid;

fn command(items: usize) -> CreateOrderCommand {
    let price = Decimal::new(1250, 2);
    let quantity = 2;
    let sub_total = price * Decimal::from(quantity);
    CreateOrderCommand::new(
        Uuid::new_v4(),
        sub_total * Decimal::from(items),
        (0..items)
            .map(|_| CreateOrderItemDto::new(Uuid::new_v4(), quantity, price, sub_total))
            .collect(),
    )
}

fn create_order(c: &mut Criterion) {
    let mut group = c.benchmark_group("create_order");

    for items in [1, 10] {
        group.bench_with_input(BenchmarkId::new("map", items), &items, |b, &items| {
            b.iter_batched(
                || command(items),
                |command| {
                    OrderMapper::map_create_order_command_to_domain_entity(black_box(command))
                },
                criterion::BatchSize::SmallInput,
            )
        });

        let order = OrderMapper::map_create_order_command_to_domain_entity(command(items));
        group.bench_with_input(BenchmarkId::new("validate", items), &order, |b, order| {
            b.iter(|| black_box(order).validate().unwrap())
        });
    }

    group.finish();
}

criterion_group!(benches, create_order);
criterion_main!(benches);
