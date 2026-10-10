CREATE TABLE outbox_messages (
    id uuid PRIMARY KEY,
    saga_id uuid NOT NULL,
    message_type VARCHAR(64) NOT NULL,
    payload JSONB NOT NULL,
    outbox_status VARCHAR(16) NOT NULL CHECK (outbox_status IN ('STARTED', 'COMPLETED', 'FAILED')),
    attempts INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_at TIMESTAMP
);
CREATE INDEX idx_outbox_messages_status_created_at ON outbox_messages (outbox_status, created_at);
