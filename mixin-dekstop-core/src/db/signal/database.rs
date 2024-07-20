use std::error::Error;

use sqlx::{ConnectOptions, Pool, Sqlite};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

pub struct SignalDatabase {
    pub(crate) pool: Pool<Sqlite>,
}

impl SignalDatabase {
    async fn connect(identity_number: String) -> Result<Self, Box<dyn Error>> {
        let pool = SqlitePoolOptions::new()
            .connect_with(SqliteConnectOptions::new()
                .filename("signal.db")
                .create_if_missing(true)).await?;
        let migrator = sqlx::migrate!("./src/db/signal/migrations");
        migrator.run(&pool).await?;
        Ok(SignalDatabase { pool })
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
        db.save_pre_key(0, &PreKeyRecord::new(0, &key_pair), None).await.expect("save prekey error");
        let a = db.get_pre_key(0, None).await.expect("get prekey error");
        println!("{:?}", a.id());
        Ok(())
    }
}