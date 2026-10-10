ALTER TABLE orders ADD COLUMN failure_messages TEXT[] NOT NULL DEFAULT '{}';
CREATE UNIQUE INDEX idx_orders_tracking_id ON orders (tracking_id);
