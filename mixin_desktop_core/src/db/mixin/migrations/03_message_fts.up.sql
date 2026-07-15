CREATE VIRTUAL TABLE IF NOT EXISTS message_fts USING fts5(
    message_id UNINDEXED,
    conversation_id UNINDEXED,
    content,
    tokenize = "unicode61 remove_diacritics 2 categories 'Co L* N* S*'"
);

WITH RECURSIVE searchable_messages AS (
    SELECT m.message_id,
           m.conversation_id,
           CASE
               WHEN m.category LIKE '%_TEXT' OR m.category LIKE '%_POST'
                   THEN m.content
               WHEN m.category LIKE '%_DATA'
                   THEN m.name
               WHEN m.category LIKE '%_CONTACT'
                   THEN shared_user.full_name
               WHEN m.category = 'APP_CARD' AND json_valid(m.content)
                   THEN trim(coalesce(json_extract(m.content, '$.title'), '') || ' ' ||
                             coalesce(json_extract(m.content, '$.description'), ''))
               WHEN m.category LIKE '%_TRANSCRIPT'
                   THEN (
                       SELECT group_concat(
                           CASE
                               WHEN tm.category LIKE '%_TEXT' OR tm.category LIKE '%_POST'
                                   THEN tm.content
                               WHEN tm.category LIKE '%_DATA'
                                   THEN tm.media_name
                               WHEN tm.category LIKE '%_CONTACT'
                                   THEN transcript_user.full_name
                               ELSE NULL
                           END,
                           ' '
                       )
                       FROM transcript_messages AS tm
                       LEFT JOIN users AS transcript_user
                           ON transcript_user.user_id = tm.shared_user_id
                       WHERE tm.transcript_id = m.message_id
                   )
               ELSE NULL
           END AS fts_content
    FROM messages AS m
    LEFT JOIN users AS shared_user ON shared_user.user_id = m.shared_user_id
    WHERE m.status NOT IN ('UNKNOWN', 'FAILED')
),
normalized_messages (
    message_id, conversation_id, raw_content, position, previous_ascii, normalized_content
) AS (
    SELECT message_id, conversation_id, fts_content, 1, 0, ''
    FROM searchable_messages
    WHERE fts_content IS NOT NULL AND trim(fts_content) != ''

    UNION ALL

    SELECT message_id,
           conversation_id,
           raw_content,
           position + 1,
           substr(raw_content, position, 1) GLOB '[A-Za-z0-9]',
           normalized_content ||
               CASE
                   WHEN previous_ascii = 1 AND
                        substr(raw_content, position, 1) NOT GLOB '[A-Za-z0-9]'
                       THEN ' '
                   ELSE ''
               END ||
               substr(raw_content, position, 1) ||
               CASE
                   WHEN substr(raw_content, position, 1) GLOB '[A-Za-z0-9]'
                       THEN ''
                   ELSE ' '
               END
    FROM normalized_messages
    WHERE position <= length(raw_content)
)
INSERT INTO message_fts (message_id, conversation_id, content)
SELECT message_id, conversation_id, trim(normalized_content)
FROM normalized_messages
WHERE position > length(raw_content);
