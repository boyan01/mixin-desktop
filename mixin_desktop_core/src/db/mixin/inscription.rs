use sdk::{InscriptionCollection, InscriptionItem};

use crate::db::Error;

#[derive(Clone)]
pub struct InscriptionDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

impl InscriptionDao {
    pub async fn find_item(&self, hash: &str) -> Result<Option<InscriptionItem>, Error> {
        Ok(sqlx::query_as::<_, InscriptionItem>(
            "SELECT * FROM inscription_items WHERE inscription_hash = ?",
        )
        .bind(hash)
        .fetch_optional(&self.0)
        .await?)
    }

    pub async fn find_collection(
        &self,
        hash: &str,
    ) -> Result<Option<InscriptionCollection>, Error> {
        Ok(sqlx::query_as::<_, InscriptionCollection>(
            "SELECT * FROM inscription_collections WHERE collection_hash = ?",
        )
        .bind(hash)
        .fetch_optional(&self.0)
        .await?)
    }

    pub async fn insert_item(&self, item: &InscriptionItem) -> Result<(), Error> {
        sqlx::query(
            r#"INSERT OR REPLACE INTO inscription_items
               (inscription_hash, collection_hash, sequence, content_type, content_url,
                occupied_by, occupied_at, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&item.inscription_hash)
        .bind(&item.collection_hash)
        .bind(item.sequence)
        .bind(&item.content_type)
        .bind(&item.content_url)
        .bind(&item.occupied_by)
        .bind(&item.occupied_at)
        .bind(item.created_at)
        .bind(item.updated_at)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn insert_collection(&self, collection: &InscriptionCollection) -> Result<(), Error> {
        sqlx::query(
            r#"INSERT OR REPLACE INTO inscription_collections
               (collection_hash, supply, unit, symbol, name, icon_url, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&collection.collection_hash)
        .bind(&collection.supply)
        .bind(&collection.unit)
        .bind(&collection.symbol)
        .bind(&collection.name)
        .bind(&collection.icon_url)
        .bind(collection.created_at)
        .bind(collection.updated_at)
        .execute(&self.0)
        .await?;
        Ok(())
    }
}
