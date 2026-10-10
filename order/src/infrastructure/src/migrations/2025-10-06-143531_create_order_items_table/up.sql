CREATE TABLE order_items (
    id uuid PRIMARY KEY,
    order_id uuid NOT NULL REFERENCES orders(id) ON DELETE RESTRICT ON UPDATE CASCADE,
    product_id uuid NOT NULL,
    quantity INT NOT NULL CHECK (quantity >= 0),
    price NUMERIC(12,2) NOT NULL CHECK (price >= 0),
    sub_total NUMERIC(12,2) NOT NULL CHECK (price >= 0),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
)