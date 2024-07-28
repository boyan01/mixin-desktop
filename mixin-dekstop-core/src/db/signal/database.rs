use std::error::Error;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::db::signal::identity::IdentityDao;
use crate::db::signal::pre_key::PreKeyDao;
use crate::db::signal::sender_key::SenderKeyDao;
use crate::db::signal::session::SessionDao;
use crate::db::signal::signed_pre_key::SignedPreKeyDao;

pub struct SignalDatabase {
    pub pre_key_dao: PreKeyDao,
    pub signed_pre_key_dao: SignedPreKeyDao,
    pub session_dao: SessionDao,
    pub sender_key_dao: SenderKeyDao,
    pub identity_dao: IdentityDao,
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
            session_dao: SessionDao(pool.clone()),
            sender_key_dao: SenderKeyDao(pool.clone()),
            identity_dao: IdentityDao(pool),
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
        let db = SignalDatabase::connect("".to_string()).await?;
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
        println!("{:x?}", a.unwrap());
        Ok(())
    }
}
