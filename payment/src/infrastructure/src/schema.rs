// @generated automatically by Diesel CLI.

diesel::table! {
    outbox_messages (id) {
        id -> Uuid,
        saga_id -> Uuid,
        #[max_length = 64]
        message_type -> Varchar,
        payload -> Jsonb,
        #[max_length = 16]
        outbox_status -> Varchar,
        attempts -> Int4,
        created_at -> Timestamp,
        processed_at -> Nullable<Timestamp>,
        #[max_length = 128]
        trace_context -> Nullable<Varchar>,
    }
}

diesel::table! {
    payments (id) {
        id -> Uuid,
        order_id -> Uuid,
        customer_id -> Uuid,
        price -> Numeric,
        #[max_length = 50]
        payment_status -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        failure_messages -> Array<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(outbox_messages, payments,);
