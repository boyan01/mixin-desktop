CREATE TABLE IF NOT EXISTS auths
(
    user_id     TEXT NOT NULL,
    private_key BLOB NOT NULL,
    account     TEXT NOT NULL,
    PRIMARY KEY (user_id)
);

CREATE TABLE IF NOT EXISTS properties
(
    "key"   TEXT NOT NULL,
    "group" TEXT NOT NULL,
    "value" TEXT NOT NULL,
    PRIMARY KEY ("key", "group")
);
