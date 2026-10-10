CREATE TABLE payments (
    id uuid PRIMARY KEY,
    order_id uuid NOT NULL,
    customer_id uuid NOT NULL,
    price NUMERIC(19, 4) NOT NULL,
    payment_status VARCHAR(50) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
)

