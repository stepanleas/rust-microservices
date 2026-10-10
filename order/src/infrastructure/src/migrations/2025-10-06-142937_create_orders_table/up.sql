CREATE TABLE orders (
    id uuid PRIMARY KEY,
    customer_id uuid NOT NULL REFERENCES customers(id) ON DELETE RESTRICT ON UPDATE CASCADE,
    tracking_id uuid NOT NULL,
    price NUMERIC(10, 2) NOT NULL,
    status VARCHAR(10) NOT NULL CHECK (status IN ('pending', 'paid', 'approved', 'cancelling', 'cancelled')),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
)