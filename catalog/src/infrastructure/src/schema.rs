// @generated automatically by Diesel CLI.

diesel::table! {
    categories (id) {
        id -> Uuid,
        #[max_length = 255]
        title -> Varchar,
        description -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    products (id) {
        id -> Uuid,
        category_id -> Uuid,
        #[max_length = 255]
        title -> Varchar,
        description -> Text,
        quantity -> Int4,
        price -> Numeric,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(products -> categories (category_id));

diesel::allow_tables_to_appear_in_same_query!(categories, products,);
