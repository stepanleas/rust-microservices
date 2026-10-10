CREATE TABLE products (
    id uuid PRIMARY KEY,
    category_id uuid NOT NULL REFERENCES categories(id) ON DELETE RESTRICT ON UPDATE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    quantity INT NOT NULL CHECK (quantity >= 0),
    price NUMERIC(12,2) NOT NULL CHECK (price >= 0),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
)