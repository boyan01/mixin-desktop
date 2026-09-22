use chrono::{DateTime, Utc};

use crate::db;

pub struct Identity {
    pub address: String,
    pub registration_id: Option<u32>,
    pub public_key: Vec<u8>,
    pub private_key: Option<Vec<u8>>,
    pub timestamp: DateTime<Utc>,
}

pub struct IdentityDao(pub(crate) crate::db::Pool);

impl IdentityDao {
    pub async fn find_identity_by_address(
        &self,
        address: &str,
    ) -> Result<Option<Identity>, db::Error> {
        let result = crate::db::query_as::<Identity>("SELECT * FROM identities WHERE address = ?")
            .bind(address)
            .fetch_optional(&self.0)
            .await?;
        Ok(result)
    }

    pub async fn get_local_identity(&self) -> Result<Option<Identity>, db::Error> {
        let result =
            crate::db::query_as::<Identity>("SELECT * FROM identities WHERE address = '-1'")
                .fetch_optional(&self.0)
                .await?;
        Ok(result)
    }

    pub async fn save_identity(&self, identity: &Identity) -> Result<(), db::Error> {
        let _ = crate::db::query(
            "INSERT OR REPLACE INTO identities (address, registration_id, public_key, private_key, timestamp) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&identity.address)
        .bind(identity.registration_id)
        .bind(&identity.public_key)
        .bind(&identity.private_key)
        .bind(identity.timestamp.timestamp_millis())
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn delete_identity(&self, address: &str) -> Result<(), db::Error> {
        let _ = crate::db::query("DELETE FROM identities WHERE address = ?")
            .bind(address)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}

crate::db::storage::row_mapping!(Identity {
    address: "address",
    registration_id: "registration_id",
    public_key: "public_key",
    private_key: "private_key",
    timestamp: "timestamp" => crate::db::datetime::DatabaseDateTime,
});
