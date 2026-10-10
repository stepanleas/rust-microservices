CREATE TABLE order_approvals (
    id uuid PRIMARY KEY,
    saga_id uuid NOT NULL,
    order_id uuid NOT NULL UNIQUE,
    status VARCHAR(16) NOT NULL CHECK (status IN ('APPROVED', 'REJECTED')),
    failure_messages TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);
