-- This file should undo anything in `up.sql`
SELECT 'DROP TABLE IF EXISTS ' || name || ';'
FROM sqlite_master
WHERE type = 'table';