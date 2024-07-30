pub struct MessageMentionDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);


#[derive(Debug, PartialEq, Eq)]
#[derive(sqlx::FromRow)]
pub struct MessageMention {
    pub message_mention_id: String,
    pub message_id: String,
    pub has_read: bool,
}