-- Current signal.db schema (v1).

CREATE TABLE sender_keys (
    group_id TEXT NOT NULL,
    sender_id TEXT NOT NULL,
    record BLOB NOT NULL,
    PRIMARY KEY (group_id, sender_id)
);

CREATE TABLE identities (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    address TEXT NOT NULL,
    registration_id INTEGER,
    public_key BLOB NOT NULL,
    private_key BLOB,
    next_prekey_id INTEGER,
    timestamp INTEGER NOT NULL
);
CREATE UNIQUE INDEX index_identities_address ON identities (address);

CREATE TABLE prekeys (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    prekey_id INTEGER NOT NULL,
    record BLOB NOT NULL
);
CREATE UNIQUE INDEX index_prekeys_prekey_id ON prekeys (prekey_id);

CREATE TABLE signed_prekeys (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    prekey_id INTEGER NOT NULL,
    record BLOB NOT NULL,
    timestamp INTEGER NOT NULL
);
CREATE UNIQUE INDEX index_signed_prekeys_prekey_id ON signed_prekeys (prekey_id);

CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    address TEXT NOT NULL,
    device INTEGER NOT NULL,
    record BLOB NOT NULL,
    timestamp INTEGER NOT NULL
);
CREATE UNIQUE INDEX index_sessions_address_device ON sessions (address, device);

CREATE TABLE ratchet_sender_keys (
    group_id TEXT NOT NULL,
    sender_id TEXT NOT NULL,
    status TEXT NOT NULL,
    message_id TEXT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (group_id, sender_id)
);

CREATE TABLE properties (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
