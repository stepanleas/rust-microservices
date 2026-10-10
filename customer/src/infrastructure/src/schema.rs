// @generated automatically by Diesel CLI.

diesel::table! {
    customers (id) {
        id -> Uuid,
        #[max_length = 255]
        user_name -> Varchar,
        #[max_length = 255]
        first_name -> Varchar,
        #[max_length = 255]
        last_name -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}
