CREATE TABLE IF NOT EXISTS sender_keys
(
    group_id  TEXT NOT NULL,
    sender_id TEXT NOT NULL,
    record    BLOB NOT NULL,
    PRIMARY KEY (group_id, sender_id)
);

CREATE TABLE IF NOT EXISTS identities
(
    address         TEXT    NOT NULL,
    registration_id INTEGER,
    public_key      BLOB    NOT NULL,
    private_key     BLOB,
    next_prekey_id  INTEGER,
    timestamp       INTEGER NOT NULL,
    PRIMARY KEY (address)
);


CREATE TABLE IF NOT EXISTS prekeys
(
    prekey_id INTEGER NOT NULL,
    record    BLOB    NOT NULL,
    PRIMARY KEY (prekey_id)
);

CREATE TABLE IF NOT EXISTS signed_prekeys
(
    prekey_id INTEGER NOT NULL,
    record    BLOB    NOT NULL,
    timestamp INTEGER NOT NULL,
    PRIMARY KEY (prekey_id)
);


CREATE TABLE IF NOT EXISTS sessions
(
    address   TEXT    NOT NULL,
    device    INTEGER NOT NULL,
    record    BLOB    NOT NULL,
    timestamp INTEGER NOT NULL,
    PRIMARY KEY (address, device)
);

CREATE TABLE IF NOT EXISTS ratchet_sender_keys
(
    group_id   TEXT NOT NULL,
    sender_id  TEXT NOT NULL,
    status     TEXT NOT NULL,
    message_id TEXT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (group_id, sender_id)
);


CREATE TABLE IF NOT EXISTS properties
(
    "key"   TEXT NOT NULL,
    "group" TEXT NOT NULL,
    "value" TEXT NOT NULL,
    PRIMARY KEY ("key", "group")
);
