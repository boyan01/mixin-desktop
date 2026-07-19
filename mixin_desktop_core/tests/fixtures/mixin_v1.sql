-- Historical Flutter mixin.db v1 schema used by migration tests.

CREATE TABLE addresses (
  address_id TEXT NOT NULL, type TEXT NOT NULL, asset_id TEXT NOT NULL,
  public_key TEXT, label TEXT, updated_at INTEGER NOT NULL, reserve TEXT NOT NULL,
  fee TEXT NOT NULL, account_name TEXT, account_tag TEXT, dust TEXT,
  PRIMARY KEY(address_id)
);
CREATE TABLE apps (
  app_id TEXT NOT NULL, app_number TEXT NOT NULL, home_uri TEXT NOT NULL,
  redirect_uri TEXT NOT NULL, name TEXT NOT NULL, icon_url TEXT NOT NULL,
  category TEXT, description TEXT NOT NULL, app_secret TEXT NOT NULL,
  capabilities TEXT, creator_id TEXT NOT NULL, resource_patterns TEXT,
  updated_at INTEGER, PRIMARY KEY(app_id)
);
CREATE TABLE assets (
  asset_id TEXT NOT NULL, symbol TEXT NOT NULL, name TEXT NOT NULL,
  icon_url TEXT NOT NULL, balance TEXT NOT NULL, destination TEXT NOT NULL,
  tag TEXT, price_btc TEXT NOT NULL, price_usd TEXT NOT NULL,
  chain_id TEXT NOT NULL, change_usd TEXT NOT NULL, change_btc TEXT NOT NULL,
  confirmations INTEGER NOT NULL, asset_key TEXT, PRIMARY KEY(asset_id)
);
CREATE TABLE circle_conversations (
  conversation_id TEXT NOT NULL, circle_id TEXT NOT NULL, user_id TEXT,
  created_at INTEGER NOT NULL, pin_time INTEGER,
  PRIMARY KEY(conversation_id, circle_id)
);
CREATE TABLE circles (
  circle_id TEXT NOT NULL, name TEXT NOT NULL, created_at INTEGER NOT NULL,
  ordered_at INTEGER, PRIMARY KEY(circle_id)
);
CREATE TABLE conversations (
  conversation_id TEXT NOT NULL, owner_id TEXT, category TEXT, name TEXT,
  icon_url TEXT, announcement TEXT, code_url TEXT, pay_type TEXT,
  created_at INTEGER NOT NULL, pin_time INTEGER, last_message_id TEXT,
  last_message_created_at INTEGER, last_read_message_id TEXT,
  unseen_message_count INTEGER, status INTEGER NOT NULL, draft TEXT,
  mute_until INTEGER, PRIMARY KEY(conversation_id)
);
CREATE TABLE flood_messages (
  message_id TEXT NOT NULL, data TEXT NOT NULL, created_at INTEGER NOT NULL,
  PRIMARY KEY(message_id)
);
CREATE TABLE hyperlinks (
  hyperlink TEXT NOT NULL, site_name TEXT NOT NULL, site_title TEXT NOT NULL,
  site_description TEXT, site_image TEXT, PRIMARY KEY(hyperlink)
);
CREATE TABLE jobs (
  job_id TEXT NOT NULL, "action" TEXT NOT NULL, created_at INTEGER NOT NULL,
  order_id INTEGER, priority INTEGER NOT NULL, user_id TEXT, blaze_message TEXT,
  conversation_id TEXT, resend_message_id TEXT, run_count INTEGER NOT NULL,
  PRIMARY KEY(job_id)
);
CREATE TABLE message_mentions (
  message_id TEXT NOT NULL, conversation_id TEXT NOT NULL, has_read BOOLEAN,
  PRIMARY KEY(message_id)
);
CREATE TABLE messages (
  message_id TEXT NOT NULL, conversation_id TEXT NOT NULL, user_id TEXT NOT NULL,
  category TEXT NOT NULL, content TEXT, media_url TEXT, media_mime_type TEXT,
  media_size INTEGER, media_duration TEXT, media_width INTEGER,
  media_height INTEGER, media_hash TEXT, thumb_image TEXT, media_key TEXT,
  media_digest TEXT, media_status TEXT, status TEXT NOT NULL,
  created_at INTEGER NOT NULL, "action" TEXT, participant_id TEXT,
  snapshot_id TEXT, hyperlink TEXT, name TEXT, album_id TEXT, sticker_id TEXT,
  shared_user_id TEXT, media_waveform TEXT, quote_message_id TEXT,
  quote_content TEXT, thumb_url TEXT, PRIMARY KEY(message_id),
  FOREIGN KEY(conversation_id) REFERENCES conversations(conversation_id)
    ON UPDATE NO ACTION ON DELETE CASCADE
);
CREATE VIRTUAL TABLE messages_fts USING FTS5(
  message_id UNINDEXED, conversation_id UNINDEXED, content,
  created_at UNINDEXED, user_id UNINDEXED, reserved_int UNINDEXED,
  reserved_text UNINDEXED, tokenize='unicode61'
);
CREATE TABLE messages_history (message_id TEXT NOT NULL, PRIMARY KEY(message_id));
CREATE TABLE offsets ("key" TEXT NOT NULL, timestamp TEXT NOT NULL, PRIMARY KEY("key"));
CREATE TABLE participant_session (
  conversation_id TEXT NOT NULL, user_id TEXT NOT NULL, session_id TEXT NOT NULL,
  sent_to_server INTEGER, created_at INTEGER, public_key TEXT,
  PRIMARY KEY(conversation_id, user_id, session_id)
);
CREATE TABLE participants (
  conversation_id TEXT NOT NULL, user_id TEXT NOT NULL, role TEXT,
  created_at INTEGER NOT NULL, PRIMARY KEY(conversation_id,user_id),
  FOREIGN KEY(conversation_id) REFERENCES conversations(conversation_id)
    ON UPDATE NO ACTION ON DELETE CASCADE
);
CREATE TABLE resend_session_messages (
  message_id TEXT NOT NULL, user_id TEXT NOT NULL, session_id TEXT NOT NULL,
  status INTEGER NOT NULL, created_at INTEGER NOT NULL,
  PRIMARY KEY(message_id, user_id, session_id)
);
CREATE TABLE sent_session_sender_keys (
  conversation_id TEXT NOT NULL, user_id TEXT NOT NULL, session_id TEXT NOT NULL,
  sent_to_server INTEGER NOT NULL, sender_key_id INTEGER, created_at INTEGER,
  PRIMARY KEY(conversation_id,user_id,session_id)
);
CREATE TABLE snapshots (
  snapshot_id TEXT NOT NULL, type TEXT NOT NULL, asset_id TEXT NOT NULL,
  amount TEXT NOT NULL, created_at INTEGER NOT NULL, opponent_id TEXT,
  transaction_hash TEXT, sender TEXT, receiver TEXT, memo TEXT,
  confirmations INTEGER, PRIMARY KEY(snapshot_id)
);
CREATE TABLE sticker_albums (
  album_id TEXT NOT NULL, name TEXT NOT NULL, icon_url TEXT NOT NULL,
  created_at INTEGER NOT NULL, update_at INTEGER NOT NULL, user_id TEXT NOT NULL,
  category TEXT NOT NULL, description TEXT NOT NULL, PRIMARY KEY(album_id)
);
CREATE TABLE sticker_relationships (
  album_id TEXT NOT NULL, sticker_id TEXT NOT NULL,
  PRIMARY KEY(album_id,sticker_id)
);
CREATE TABLE stickers (
  sticker_id TEXT NOT NULL, album_id TEXT, name TEXT NOT NULL,
  asset_url TEXT NOT NULL, asset_type TEXT NOT NULL, asset_width INTEGER NOT NULL,
  asset_height INTEGER NOT NULL, created_at INTEGER NOT NULL, last_use_at INTEGER,
  PRIMARY KEY(sticker_id)
);
CREATE TABLE users (
  user_id TEXT NOT NULL, identity_number TEXT NOT NULL, relationship TEXT,
  full_name TEXT, avatar_url TEXT, phone TEXT, is_verified BOOLEAN,
  created_at INTEGER, mute_until INTEGER, has_pin INTEGER, app_id TEXT,
  biography TEXT, is_scam INTEGER, PRIMARY KEY(user_id)
);
CREATE INDEX index_jobs_action ON jobs("action");
CREATE INDEX index_conversations_category_status_pin_time_created_at
  ON conversations(category, status, pin_time, created_at);
CREATE INDEX index_messages_conversation_id ON messages(conversation_id);
CREATE INDEX index_messages_conversation_id_created_at
  ON messages(conversation_id, created_at);
CREATE INDEX index_messages_conversation_id_status_user_id
  ON messages(conversation_id, status, user_id);
CREATE INDEX index_messages_conversation_id_user_id_status_created_at
  ON messages(conversation_id, user_id, status, created_at);
CREATE INDEX index_participants_conversation_id ON participants(conversation_id);
CREATE INDEX index_participants_created_at ON participants(created_at);
CREATE INDEX index_snapshots_asset_id ON snapshots(asset_id);
CREATE INDEX index_users_full_name ON users(full_name);
CREATE TRIGGER conversation_last_message_update AFTER INSERT ON messages BEGIN
  UPDATE conversations SET last_message_id = new.message_id,
    last_message_created_at = new.created_at
  WHERE conversation_id = new.conversation_id;
END;
CREATE TRIGGER conversation_last_message_delete AFTER DELETE ON messages BEGIN
  UPDATE conversations SET last_message_id = (
    SELECT message_id FROM messages WHERE conversation_id = old.conversation_id
    ORDER BY created_at DESC LIMIT 1
  ) WHERE conversation_id = old.conversation_id;
END;
