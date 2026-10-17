//! How much CPU a saga message costs to serialize: `cargo bench --features contracts --bench avro`.
//!
//! `AvroMessage::to_avro_bytes` parses the schema on every call and writes an object container
//! with the schema embedded, so `encode` includes both; `schema_parse` isolates the parsing.
use std::hint::black_box;

use apache_avro::Schema;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use shared::contracts::AvroMessage;
use shared::contracts::restaurant::{
    RestaurantApprovalProductAvroModel, RestaurantApprovalRequestAvroModel,
};

fn request(products: usize) -> RestaurantApprovalRequestAvroModel {
    RestaurantApprovalRequestAvroModel {
        id: "6f1c1f0e-3b8e-4a43-9d0e-2a4b1c7d9e10".into(),
        saga_id: "0411a4a9-1edb-4180-9556-800cb2b84721".into(),
        order_id: "9b2f7c1e-5d4a-4e8b-a1c3-7f6e5d4c3b2a".into(),
        customer_id: "c3b2a1f0-9e8d-4c7b-b6a5-0f1e2d3c4b5a".into(),
        price: "250.00".into(),
        products: (0..products)
            .map(|i| RestaurantApprovalProductAvroModel {
                id: format!("00000000-0000-4000-8000-{i:012}"),
                quantity: 1,
            })
            .collect(),
        created_at: "2026-10-17T12:00:00Z".into(),
    }
}

fn avro(c: &mut Criterion) {
    let mut group = c.benchmark_group("restaurant_approval_request");

    group.bench_function("schema_parse", |b| {
        b.iter(|| Schema::parse_str(black_box(RestaurantApprovalRequestAvroModel::SCHEMA)).unwrap())
    });

    for products in [1, 10] {
        let message = request(products);
        let bytes = message.to_avro_bytes().unwrap();
        group.throughput(Throughput::Bytes(bytes.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("encode", products),
            &message,
            |b, message| b.iter(|| black_box(message).to_avro_bytes().unwrap()),
        );
        group.bench_with_input(BenchmarkId::new("decode", products), &bytes, |b, bytes| {
            b.iter(|| {
                RestaurantApprovalRequestAvroModel::from_avro_bytes(black_box(bytes)).unwrap()
            })
        });
    }

    group.finish();
}

criterion_group!(benches, avro);
criterion_main!(benches);
