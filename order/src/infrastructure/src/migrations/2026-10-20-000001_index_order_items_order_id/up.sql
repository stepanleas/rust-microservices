-- Loading an order reads its items by order_id; without this index every lookup scanned the whole
-- table (on the lab at 150 req/s: ~0.5 s per query and an exhausted connection pool).
-- IF NOT EXISTS: the lab got it by hand before this release.
CREATE INDEX IF NOT EXISTS idx_order_items_order_id ON order_items (order_id);
