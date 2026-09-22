use crate::db;

pub struct SenderKeyDao(pub(crate) crate::db::Pool);

impl SenderKeyDao {
    pub async fn find_sender_key(
        &self,
        group_id: &str,
        sender_name: &str,
        device_id: u32,
    ) -> Result<Option<Vec<u8>>, db::Error> {
        let result = crate::db::query_scalar::<Vec<u8>>(
            "SELECT record FROM sender_keys WHERE group_id = ? AND sender_id = ?",
        )
        .bind(group_id)
        .bind(format!("{}:{}", sender_name, device_id))
        .fetch_optional(&self.0)
        .await?;
        Ok(result)
    }

    pub async fn save_sender_key(
        &self,
        group_id: &str,
        sender_name: &str,
        device_id: u32,
        record: Vec<u8>,
    ) -> Result<(), db::Error> {
        let _ = crate::db::query(
            "INSERT OR REPLACE INTO sender_keys (group_id, sender_id, record) VALUES (?, ?, ?)",
        )
        .bind(group_id)
        .bind(format!("{}:{}", sender_name, device_id))
        .bind(record)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn delete_sender_key(
        &self,
        group_id: &str,
        sender_name: &str,
        device_id: u32,
    ) -> Result<(), db::Error> {
        crate::db::query("DELETE FROM sender_keys WHERE group_id = ? AND sender_id = ?")
            .bind(group_id)
            .bind(format!("{}:{}", sender_name, device_id))
            .execute(&self.0)
            .await?;
        Ok(())
    }

    pub(crate) async fn has_sender_key(
        &self,
        group_id: &str,
        sender_id: &str,
        device_id: u32,
    ) -> Result<bool, db::Error> {
        let result = crate::db::query_scalar::<bool>(
            "SELECT EXISTS(SELECT 1 FROM sender_keys WHERE group_id = ? AND sender_id = ?)",
        )
        .bind(group_id)
        .bind(format!("{}:{}", sender_id, device_id))
        .fetch_one(&self.0)
        .await?;
        Ok(result)
    }
}
