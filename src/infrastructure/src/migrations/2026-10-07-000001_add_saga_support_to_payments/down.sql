ALTER TABLE payments DROP CONSTRAINT payments_order_id_unique;

ALTER TABLE payments DROP COLUMN failure_messages;
