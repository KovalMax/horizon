-- no-transaction
CREATE INDEX CONCURRENTLY web_push_outbox_expiry_idx ON web_push_outbox (expires_at, id);
