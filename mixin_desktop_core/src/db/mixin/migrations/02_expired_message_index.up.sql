CREATE INDEX IF NOT EXISTS index_expired_messages_expire_at
    ON expired_messages (expire_at);
