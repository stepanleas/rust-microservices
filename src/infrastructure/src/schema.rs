// @generated automatically by Diesel CLI.

diesel::table! {
    order_approvals (id) {
        id -> Uuid,
        saga_id -> Uuid,
        order_id -> Uuid,
        #[max_length = 16]
        status -> Varchar,
        failure_messages -> Array<Text>,
        created_at -> Timestamp,
    }
}

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
    }
}

diesel::table! {
    products (id) {
        id -> Uuid,
        #[max_length = 255]
        title -> Varchar,
        quantity -> Int4,
        price -> Numeric,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(order_approvals, outbox_messages, products,);
