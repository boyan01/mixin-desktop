use crate::db::Error;

#[derive(Clone)]
pub struct FiatDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

impl FiatDao {
    pub async fn insert_all(&self, fiats: &[sdk::Fiat]) -> Result<(), Error> {
        let mut transaction = self.0.begin().await?;
        for fiat in fiats {
            sqlx::query("INSERT OR REPLACE INTO fiats (code, rate) VALUES (?, ?)")
                .bind(&fiat.code)
                .bind(fiat.rate)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }
}
