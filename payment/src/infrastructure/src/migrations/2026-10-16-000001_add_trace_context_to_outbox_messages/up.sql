-- W3C traceparent of the transaction that wrote the message; the outbox relay publishes the
-- message in that trace.
ALTER TABLE outbox_messages ADD COLUMN trace_context VARCHAR(128);
