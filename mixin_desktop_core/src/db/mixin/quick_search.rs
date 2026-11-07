//! Command-palette ranking. Both sources compete before applying a page limit.
use super::MixinDatabase;
use crate::db::Error;

/// `kind == 0` selects a user; `kind == 1` selects a conversation.
pub(crate) async fn ranked_ids(
    database: &MixinDatabase,
    query: &str,
    offset: i64,
    limit: i64,
) -> Result<Vec<(i64, String)>, Error> {
    let query = query.trim();
    if query.is_empty() || limit <= 0 {
        return Ok(Vec::new());
    }
    // The reference separates only ASCII spaces and considers its first three
    // nonempty words. Bound LIKE patterns preserve its explicit ESCAPE semantics.
    let mut words = query.split(' ').filter(|word| !word.is_empty());
    let first = words.next().unwrap_or_default();
    let second = words.next().unwrap_or_default();
    let third = words.next().unwrap_or_default();
    Ok(crate::db::query_as::<(i64, String)>(
        r#"
WITH keywords AS (SELECT ?1 AS query, ?2 AS first, ?3 AS second, ?4 AS third),
conversation_names AS (
    SELECT conversation.conversation_id AS id,
           CASE WHEN conversation.category = 'GROUP' THEN conversation.name ELSE owner.full_name END AS name,
           COALESCE(conversation.pin_time, 0) AS pin_time
    FROM conversations conversation
    INNER JOIN users owner ON owner.user_id = conversation.owner_id
    WHERE conversation.category = 'GROUP'
       OR (conversation.category = 'CONTACT' AND owner.relationship = 'STRANGER')
), ranked AS (
    SELECT 0 AS kind, users.user_id AS id, 0 AS pin_time,
           CASE
               WHEN users.full_name = kw.query COLLATE NOCASE THEN 1.0 + 1.0 / LENGTH(users.full_name)
               WHEN users.identity_number = kw.query COLLATE NOCASE THEN 0.9 + 1.0 / LENGTH(users.identity_number)
               WHEN users.full_name LIKE kw.query || '%' ESCAPE '\' COLLATE NOCASE THEN 0.6 + 1.0 / LENGTH(users.full_name)
               WHEN users.identity_number LIKE kw.query || '%' ESCAPE '\' COLLATE NOCASE THEN 0.5 + 1.0 / LENGTH(users.identity_number)
               WHEN users.full_name LIKE '%' || kw.query || '%' ESCAPE '\' COLLATE NOCASE THEN 0.3 + 1.0 / LENGTH(users.full_name)
               WHEN users.identity_number LIKE '%' || kw.query || '%' ESCAPE '\' COLLATE NOCASE THEN 0.2 + 1.0 / LENGTH(users.identity_number)
               ELSE 0.0
           END AS score
    FROM users CROSS JOIN keywords kw
    WHERE users.relationship = 'FRIEND'
      AND (users.full_name LIKE '%' || kw.query || '%' ESCAPE '\' COLLATE NOCASE
           OR users.identity_number LIKE '%' || kw.query || '%' ESCAPE '\' COLLATE NOCASE)
    UNION ALL
    SELECT 1 AS kind, c.id, c.pin_time,
           CASE
               WHEN LENGTH(kw.second) = 0 THEN
                   CASE
                       WHEN c.name = kw.query COLLATE NOCASE THEN 1.0 + 1.0 / LENGTH(c.name)
                       WHEN c.name LIKE kw.query || '%' ESCAPE '\' COLLATE NOCASE THEN 0.6 + 1.0 / LENGTH(c.name)
                       WHEN c.name LIKE '%' || kw.query || '%' ESCAPE '\' COLLATE NOCASE THEN 0.3 + 1.0 / LENGTH(c.name)
                       ELSE 0.0
                   END
               WHEN (c.name LIKE '%' || kw.first || '%' ESCAPE '\' COLLATE NOCASE)
                 OR (c.name LIKE '%' || kw.second || '%' ESCAPE '\' COLLATE NOCASE)
                 OR (LENGTH(kw.third) > 0 AND c.name LIKE '%' || kw.third || '%' ESCAPE '\' COLLATE NOCASE)
               THEN
                   ((CASE WHEN c.name LIKE '%' || kw.first || '%' ESCAPE '\' COLLATE NOCASE THEN 1 ELSE 0 END)
                    + (CASE WHEN c.name LIKE '%' || kw.second || '%' ESCAPE '\' COLLATE NOCASE THEN 1 ELSE 0 END)
                    + (CASE WHEN LENGTH(kw.third) > 0 AND c.name LIKE '%' || kw.third || '%' ESCAPE '\' COLLATE NOCASE THEN 1 ELSE 0 END)) * 0.1
                   + CASE WHEN c.name LIKE '%' || kw.first || '%' || kw.second || '%' ESCAPE '\' COLLATE NOCASE THEN 0.2 ELSE 0.0 END
                   + CASE WHEN c.name LIKE '%' || kw.first || ' ' || kw.second || '%' ESCAPE '\' COLLATE NOCASE THEN 0.15 ELSE 0.0 END
                   + CASE WHEN c.name LIKE kw.first || '%' ESCAPE '\' COLLATE NOCASE THEN 0.1 ELSE 0.0 END
                   + 1.0 / LENGTH(c.name)
               ELSE 0.0
           END AS score
    FROM conversation_names c CROSS JOIN keywords kw
)
SELECT kind, id FROM ranked
ORDER BY score DESC, kind ASC, pin_time DESC, id ASC
LIMIT ?5 OFFSET ?6
        "#,
    )
    .bind(query)
    .bind(first)
    .bind(second)
    .bind(third)
    .bind(limit.min(200))
    .bind(offset.max(0))
    .fetch_all(&database.user_dao.0)
    .await?)
}

#[cfg(test)]
mod tests;
