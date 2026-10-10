DROP INDEX IF EXISTS idx_orders_tracking_id;
ALTER TABLE orders DROP COLUMN failure_messages;
