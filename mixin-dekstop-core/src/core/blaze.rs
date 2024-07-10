use std::error::Error;
use std::sync::Arc;
use reqwest::Url;
use crate::db::MixinDatabase;
use crate::sdk::{Client, Credential};

const WS_HOST: &str = "wss: //blaze.mixin.one";

pub struct Blaze {
    database: Arc<MixinDatabase>,
    client: Client,
    credential: Credential,
}

impl Blaze {
    pub fn connect(&self) -> Result<(), Box<dyn Error>> {
        let a = Url::parse(WS_HOST)?;

        Ok(())
    }
}