use crate::db::Error;

#[derive(Clone)]
pub struct FiatDao(pub(crate) crate::db::Pool);

impl FiatDao {
    pub async fn insert_all(&self, fiats: &[sdk::Fiat]) -> Result<(), Error> {
        let mut transaction = self.0.begin_with("BEGIN IMMEDIATE").await?;
        for fiat in fiats {
            crate::db::query("INSERT OR REPLACE INTO fiats (code, rate) VALUES (?, ?)")
                .bind(&fiat.code)
                .bind(fiat.rate)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }
}
