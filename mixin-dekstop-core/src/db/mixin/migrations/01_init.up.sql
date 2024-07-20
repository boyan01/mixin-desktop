CREATE TABLE IF NOT EXISTS addresses
(
    address_id  TEXT      NOT NULL,
    type        TEXT      NOT NULL,
    asset_id    TEXT      NOT NULL,
    destination TEXT      NOT NULL,
    label       TEXT      NOT NULL,
    updated_at  TIMESTAMP NOT NULL,
    reserve     TEXT      NOT NULL,
    fee         TEXT      NOT NULL,
    tag         TEXT,
    dust        TEXT,
    PRIMARY KEY (address_id)
);

CREATE TABLE IF NOT EXISTS apps
(
    app_id            TEXT NOT NULL,
    app_number        TEXT NOT NULL,
    home_uri          TEXT NOT NULL,
    redirect_uri      TEXT NOT NULL,
    name              TEXT NOT NULL,
    icon_url          TEXT NOT NULL,
    category          TEXT,
    description       TEXT NOT NULL,
    app_secret        TEXT NOT NULL,
    capabilities      TEXT,
    creator_id        TEXT NOT NULL,
    resource_patterns TEXT,
    updated_at        TIMESTAMP,
    PRIMARY KEY (app_id)
);

CREATE TABLE IF NOT EXISTS assets
(
    asset_id      TEXT    NOT NULL,
    symbol        TEXT    NOT NULL,
    name          TEXT    NOT NULL,
    icon_url      TEXT    NOT NULL,
    balance       TEXT    NOT NULL,
    destination   TEXT    NOT NULL,
    tag           TEXT,
    price_btc     TEXT    NOT NULL,
    price_usd     TEXT    NOT NULL,
    chain_id      TEXT    NOT NULL,
    change_usd    TEXT    NOT NULL,
    change_btc    TEXT    NOT NULL,
    confirmations INTEGER NOT NULL,
    asset_key     TEXT,
    reserve       TEXT,
    PRIMARY KEY (asset_id)
);

CREATE TABLE IF NOT EXISTS circle_conversations
(
    conversation_id TEXT      NOT NULL,
    circle_id       TEXT      NOT NULL,
    user_id         TEXT,
    created_at      TIMESTAMP NOT NULL,
    pin_time        TIMESTAMP,
    PRIMARY KEY (conversation_id, circle_id)
);

CREATE TABLE IF NOT EXISTS circles
(
    circle_id  TEXT      NOT NULL,
    name       TEXT      NOT NULL,
    created_at TIMESTAMP NOT NULL,
    ordered_at TIMESTAMP,
    PRIMARY KEY (circle_id)
);

CREATE TABLE IF NOT EXISTS conversations
(
    conversation_id         TEXT      NOT NULL,
    owner_id                TEXT,
    category                TEXT,
    name                    TEXT,
    icon_url                TEXT,
    announcement            TEXT,
    code_url                TEXT,
    pay_type                TEXT,
    created_at              TIMESTAMP NOT NULL,
    pin_time                TIMESTAMP,
    last_message_id         TEXT,
    last_message_created_at INTEGER,
    last_read_message_id    TEXT,
    unseen_message_count    INTEGER,
    status                  INTEGER   NOT NULL,
    draft                   TEXT,
    mute_until              TIMESTAMP,
    expire_in               TIMESTAMP,
    PRIMARY KEY (conversation_id)
);

CREATE TABLE IF NOT EXISTS flood_messages
(
    message_id TEXT      NOT NULL,
    data       TEXT      NOT NULL,
    created_at TIMESTAMP NOT NULL,
    PRIMARY KEY (message_id)
);

CREATE TABLE IF NOT EXISTS hyperlinks
(
    hyperlink        TEXT NOT NULL,
    site_name        TEXT NOT NULL,
    site_title       TEXT NOT NULL,
    site_description TEXT,
    site_image       TEXT,
    PRIMARY KEY (hyperlink)
);

CREATE TABLE IF NOT EXISTS jobs
(
    job_id            TEXT      NOT NULL,
    "action"          TEXT      NOT NULL,
    created_at        TIMESTAMP NOT NULL,
    order_id          INTEGER,
    priority          INTEGER   NOT NULL,
    user_id           TEXT,
    blaze_message     TEXT,
    conversation_id   TEXT,
    resend_message_id TEXT,
    run_count         INTEGER   NOT NULL,
    PRIMARY KEY (job_id)
);

CREATE TABLE IF NOT EXISTS message_mentions
(
    message_id      TEXT NOT NULL,
    conversation_id TEXT NOT NULL,
    has_read        BOOLEAN,
    PRIMARY KEY (message_id)
);

CREATE TABLE IF NOT EXISTS messages
(
    message_id       TEXT      NOT NULL,
    conversation_id  TEXT      NOT NULL,
    user_id          TEXT      NOT NULL,
    category         TEXT      NOT NULL,
    content          TEXT,
    media_url        TEXT,
    media_mime_type  TEXT,
    media_size       INTEGER,
    media_duration   TEXT,
    media_width      INTEGER,
    media_height     INTEGER,
    media_hash       TEXT,
    thumb_image      TEXT,
    media_key        TEXT,
    media_digest     TEXT,
    media_status     TEXT,
    status           TEXT      NOT NULL,
    created_at       TIMESTAMP NOT NULL,
    "action"         TEXT,
    participant_id   TEXT,
    snapshot_id      TEXT,
    hyperlink        TEXT,
    name             TEXT,
    album_id         TEXT,
    sticker_id       TEXT,
    shared_user_id   TEXT,
    media_waveform   TEXT,
    quote_message_id TEXT,
    quote_content    TEXT,
    thumb_url        TEXT,
    caption          TEXT,
    PRIMARY KEY (message_id),
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id) ON UPDATE NO ACTION ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS messages_history
(
    message_id TEXT NOT NULL,
    PRIMARY KEY (message_id)
);

CREATE TABLE IF NOT EXISTS offsets
(
    "key"     TEXT      NOT NULL,
    timestamp TIMESTAMP NOT NULL,
    PRIMARY KEY ("key")
);

CREATE TABLE IF NOT EXISTS participant_session
(
    conversation_id TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    session_id      TEXT NOT NULL,
    sent_to_server  INTEGER,
    created_at      TIMESTAMP,
    public_key      TEXT,
    PRIMARY KEY (conversation_id, user_id, session_id)
);

CREATE TABLE IF NOT EXISTS participants
(
    conversation_id TEXT      NOT NULL,
    user_id         TEXT      NOT NULL,
    role            TEXT,
    created_at      TIMESTAMP NOT NULL,
    PRIMARY KEY (conversation_id, user_id),
    FOREIGN KEY (conversation_id) REFERENCES conversations (conversation_id) ON UPDATE NO ACTION ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS resend_session_messages
(
    message_id TEXT      NOT NULL,
    user_id    TEXT      NOT NULL,
    session_id TEXT      NOT NULL,
    status     INTEGER   NOT NULL,
    created_at TIMESTAMP NOT NULL,
    PRIMARY KEY (message_id, user_id, session_id)
);

CREATE TABLE IF NOT EXISTS sent_session_sender_keys
(
    conversation_id TEXT    NOT NULL,
    user_id         TEXT    NOT NULL,
    session_id      TEXT    NOT NULL,
    sent_to_server  INTEGER NOT NULL,
    sender_key_id   INTEGER,
    created_at      TIMESTAMP,
    PRIMARY KEY (conversation_id, user_id, session_id)
);

CREATE TABLE IF NOT EXISTS snapshots
(
    snapshot_id      TEXT    NOT NULL,
    trace_id         TEXT,
    type             TEXT    NOT NULL,
    asset_id         TEXT    NOT NULL,
    amount           TEXT    NOT NULL,
    created_at       INTEGER NOT NULL,
    opponent_id      TEXT,
    transaction_hash TEXT,
    sender           TEXT,
    receiver         TEXT,
    memo             TEXT,
    confirmations    INTEGER,
    snapshot_hash    TEXT,
    opening_balance  TEXT,
    closing_balance  TEXT,
    PRIMARY KEY (snapshot_id)
);

CREATE TABLE IF NOT EXISTS sticker_albums
(
    album_id    TEXT    NOT NULL,
    name        TEXT    NOT NULL,
    icon_url    TEXT    NOT NULL,
    created_at  INTEGER NOT NULL,
    update_at   INTEGER NOT NULL,
    ordered_at  INTEGER NOT NULL DEFAULT 0,
    user_id     TEXT    NOT NULL,
    category    TEXT    NOT NULL,
    description TEXT    NOT NULL,
    banner      TEXT,
    added       BOOLEAN          DEFAULT FALSE,
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (album_id)
);

CREATE TABLE IF NOT EXISTS sticker_relationships
(
    album_id   TEXT NOT NULL,
    sticker_id TEXT NOT NULL,
    PRIMARY KEY (album_id, sticker_id)
);

CREATE TABLE IF NOT EXISTS stickers
(
    sticker_id   TEXT    NOT NULL,
    album_id     TEXT,
    name         TEXT    NOT NULL,
    asset_url    TEXT    NOT NULL,
    asset_type   TEXT    NOT NULL,
    asset_width  INTEGER NOT NULL,
    asset_height INTEGER NOT NULL,
    created_at   INTEGER NOT NULL,
    last_use_at  INTEGER,
    PRIMARY KEY (sticker_id)
);

CREATE TABLE IF NOT EXISTS users
(
    user_id         TEXT NOT NULL,
    identity_number TEXT NOT NULL,
    relationship    TEXT,
    full_name       TEXT,
    avatar_url      TEXT,
    phone           TEXT,
    is_verified     BOOLEAN,
    created_at      TIMESTAMP,
    mute_until      TIMESTAMP,
    has_pin         INTEGER,
    app_id          TEXT,
    biography       TEXT,
    is_scam         INTEGER,
    code_url        TEXT,
    code_id         TEXT,
    is_deactivated  BOOLEAN,
    PRIMARY KEY (user_id)
);

CREATE TABLE IF NOT EXISTS transcript_messages
(
    transcript_id    TEXT      NOT NULL,
    message_id       TEXT      NOT NULL,
    user_id          TEXT,
    user_full_name   TEXT,
    category         TEXT      NOT NULL,
    created_at       TIMESTAMP NOT NULL,
    content          TEXT,
    media_url        TEXT,
    media_name       TEXT,
    media_size       INTEGER,
    media_width      INTEGER,
    media_height     INTEGER,
    media_mime_type  TEXT,
    media_duration   TEXT,
    media_status     TEXT,
    media_waveform   TEXT,
    thumb_image      TEXT,
    thumb_url        TEXT,
    media_key        TEXT,
    media_digest     TEXT,
    media_created_at INTEGER,
    sticker_id       TEXT,
    shared_user_id   TEXT,
    mentions         TEXT,
    quote_id         TEXT,
    quote_content    TEXT,
    caption          TEXT,
    PRIMARY KEY (transcript_id, message_id)
);

CREATE TABLE IF NOT EXISTS pin_messages
(
    message_id      TEXT      NOT NULL,
    conversation_id TEXT      NOT NULL,
    created_at      TIMESTAMP NOT NULL,
    PRIMARY KEY (message_id)
);

CREATE TABLE IF NOT EXISTS fiats
(
    code TEXT   NOT NULL,
    rate DOUBLE NOT NULL,
    PRIMARY KEY (code)
);

CREATE TABLE IF NOT EXISTS favorite_apps
(
    app_id     TEXT      NOT NULL,
    user_id    TEXT      NOT NULL,
    created_at TIMESTAMP NOT NULL,
    PRIMARY KEY (app_id, user_id)
);

CREATE TABLE IF NOT EXISTS expired_messages
(
    message_id TEXT      NOT NULL,
    expire_in  TIMESTAMP NOT NULL,
    expire_at  TIMESTAMP,
    PRIMARY KEY (message_id)
);

CREATE TABLE IF NOT EXISTS chains
(
    chain_id  TEXT    NOT NULL,
    name      TEXT    NOT NULL,
    symbol    TEXT    NOT NULL,
    icon_url  TEXT    NOT NULL,
    threshold INTEGER NOT NULL,
    PRIMARY KEY (chain_id)
);

CREATE TABLE IF NOT EXISTS properties
(
    "key"   TEXT NOT NULL,
    "group" TEXT NOT NULL,
    "value" TEXT NOT NULL,
    PRIMARY KEY ("key", "group")
);

CREATE TABLE IF NOT EXISTS safe_snapshots
(
    snapshot_id      TEXT      NOT NULL,
    type             TEXT      NOT NULL,
    asset_id         TEXT      NOT NULL,
    amount           TEXT      NOT NULL,
    user_id          TEXT      NOT NULL,
    opponent_id      TEXT      NOT NULL,
    memo             TEXT      NOT NULL,
    transaction_hash TEXT      NOT NULL,
    created_at       TIMESTAMP NOT NULL,
    trace_id         TEXT,
    confirmations    INTEGER,
    opening_balance  TEXT,
    closing_balance  TEXT,
    withdrawal       TEXT,
    deposit          TEXT,
    inscription_hash TEXT,
    PRIMARY KEY (snapshot_id)
);

CREATE TABLE IF NOT EXISTS tokens
(
    asset_id        TEXT    NOT NULL,
    kernel_asset_id TEXT    NOT NULL,
    symbol          TEXT    NOT NULL,
    name            TEXT    NOT NULL,
    icon_url        TEXT    NOT NULL,
    price_btc       TEXT    NOT NULL,
    price_usd       TEXT    NOT NULL,
    chain_id        TEXT    NOT NULL,
    change_usd      TEXT    NOT NULL,
    change_btc      TEXT    NOT NULL,
    confirmations   INTEGER NOT NULL,
    asset_key       TEXT    NOT NULL,
    dust            TEXT    NOT NULL,
    collection_hash TEXT,
    PRIMARY KEY (asset_id)
);

CREATE TABLE IF NOT EXISTS inscription_collections
(
    collection_hash TEXT      NOT NULL,
    supply          TEXT      NOT NULL,
    unit            TEXT      NOT NULL,
    symbol          TEXT      NOT NULL,
    name            TEXT      NOT NULL,
    icon_url        TEXT      NOT NULL,
    created_at      TIMESTAMP NOT NULL,
    updated_at      TIMESTAMP NOT NULL,
    PRIMARY KEY (collection_hash)
);

CREATE TABLE IF NOT EXISTS inscription_items
(
    inscription_hash TEXT      NOT NULL,
    collection_hash  TEXT      NOT NULL,
    sequence         INTEGER   NOT NULL,
    content_type     TEXT      NOT NULL,
    content_url      TEXT      NOT NULL,
    occupied_by      TEXT,
    occupied_at      TEXT,
    created_at       TIMESTAMP NOT NULL,
    updated_at       TIMESTAMP NOT NULL,
    PRIMARY KEY (inscription_hash)
);


CREATE INDEX IF NOT EXISTS index_conversations_category_status ON conversations (category, status);
CREATE INDEX IF NOT EXISTS index_conversations_mute_until ON conversations (mute_until);
CREATE INDEX IF NOT EXISTS index_flood_messages_created_at ON flood_messages (created_at);
CREATE INDEX IF NOT EXISTS index_jobs_action ON jobs ("action");
CREATE INDEX IF NOT EXISTS index_message_mentions_conversation_id_has_read ON message_mentions (conversation_id, has_read);
CREATE INDEX IF NOT EXISTS index_participants_conversation_id_created_at ON participants (conversation_id, created_at);
CREATE INDEX IF NOT EXISTS index_sticker_albums_category_created_at ON sticker_albums (category, created_at DESC);
CREATE INDEX IF NOT EXISTS index_pin_messages_conversation_id ON pin_messages (conversation_id);
CREATE INDEX IF NOT EXISTS index_users_identity_number ON users (identity_number);
CREATE INDEX IF NOT EXISTS index_messages_conversation_id_created_at ON messages (conversation_id, created_at DESC);
CREATE INDEX IF NOT EXISTS index_messages_conversation_id_category_created_at ON messages (conversation_id, category, created_at DESC);
CREATE INDEX IF NOT EXISTS index_message_conversation_id_status_user_id ON messages (conversation_id, status, user_id);
CREATE INDEX IF NOT EXISTS index_messages_conversation_id_quote_message_id ON messages (conversation_id, quote_message_id);
CREATE INDEX IF NOT EXISTS index_tokens_kernel_asset_id ON tokens (kernel_asset_id);
CREATE INDEX IF NOT EXISTS index_tokens_collection_hash ON tokens (collection_hash);