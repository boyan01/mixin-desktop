-- Current app.db schema (v1).

CREATE TABLE properties
(
    "key"   TEXT NOT NULL,
    "group" TEXT NOT NULL,
    "value" TEXT NOT NULL,
    PRIMARY KEY ("key", "group")
);
