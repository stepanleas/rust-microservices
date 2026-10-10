ALTER TABLE payments
    ADD COLUMN failure_messages TEXT[] NOT NULL DEFAULT '{}';

-- One payment per order: makes the payment-request consumer idempotent.
ALTER TABLE payments
    ADD CONSTRAINT payments_order_id_unique UNIQUE (order_id);
