use std::error::Error;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::ConnectOptions;

pub use crate::db::signal::pre_key::PreKeyDao;
pub use crate::db::signal::signed_pre_key::SignedPreKeyDao;

pub struct SignalDatabase {
    pub pre_key_dao: PreKeyDao,
    pub signed_pre_key_dao: SignedPreKeyDao,
}

impl SignalDatabase {
    pub async fn connect(identity_number: String) -> Result<Self, Box<dyn Error>> {
        let pool = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename("signal.db")
                    .create_if_missing(true),
            )
            .await?;
        let migrator = sqlx::migrate!("./src/db/signal/migrations");
        migrator.run(&pool).await?;
        Ok(SignalDatabase {
            pre_key_dao: PreKeyDao(pool.clone()),
            signed_pre_key_dao: SignedPreKeyDao(pool.clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use libsignal_protocol::{KeyPair, PreKeyRecord, PreKeyStore};
    use log::LevelFilter;
    use rand_core::OsRng;
    use simplelog::{Config, TestLogger};

    use super::*;

    #[tokio::test]
    async fn test_connect() -> Result<(), Box<dyn Error>> {
        let _ = TestLogger::init(LevelFilter::Info, Config::default());
        let mut db = SignalDatabase::connect("".to_string()).await?;
        let mut csprng = OsRng;
        let key_pair = KeyPair::generate(&mut csprng);
        db.pre_key_dao
            .save_pre_key(0, PreKeyRecord::new(0, &key_pair).serialize().unwrap())
            .await
            .expect("save prekey error");
        let a = db
            .pre_key_dao
            .find_pre_key(0)
            .await
            .expect("get prekey error");
        println!("{:?}", a.unwrap().record);
        Ok(())
    }
}
