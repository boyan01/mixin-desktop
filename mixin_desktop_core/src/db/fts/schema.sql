-- Current fts.db schema (v1).

CREATE VIRTUAL TABLE messages_fts USING FTS5(
    content,
    tokenize="unicode61 remove_diacritics 2 categories 'Co L* N* S*'"
);

CREATE TABLE messages_metas (
    doc_id INTEGER NOT NULL,
    message_id TEXT NOT NULL,
    conversation_id TEXT NOT NULL,
    category TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (message_id)
);

CREATE INDEX messages_metas_doc_id_created_at
    ON messages_metas (doc_id, created_at);
CREATE INDEX messages_metas_conversation_id_user_id_category
    ON messages_metas (conversation_id, user_id, category);
