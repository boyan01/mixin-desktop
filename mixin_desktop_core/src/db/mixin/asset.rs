use sdk::{Asset, Chain, Token};

use crate::db::Error;

#[derive(Clone)]
pub struct AssetDao(pub(crate) sqlx::Pool<sqlx::Sqlite>);

impl AssetDao {
    pub async fn insert_asset(&self, asset: &Asset) -> Result<(), Error> {
        sqlx::query(
            r#"INSERT OR REPLACE INTO assets
               (asset_id, symbol, name, icon_url, balance, destination, tag, price_btc,
                price_usd, chain_id, change_usd, change_btc, confirmations, asset_key, reserve)
               VALUES (?, ?, ?, ?, ?, '', ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&asset.asset_id)
        .bind(&asset.symbol)
        .bind(&asset.name)
        .bind(&asset.icon_url)
        .bind(&asset.balance)
        .bind(&asset.tag)
        .bind(&asset.price_btc)
        .bind(&asset.price_usd)
        .bind(&asset.chain_id)
        .bind(&asset.change_usd)
        .bind(&asset.change_btc)
        .bind(asset.confirmations)
        .bind(&asset.asset_key)
        .bind(&asset.reserve)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn insert_chain(&self, chain: &Chain) -> Result<(), Error> {
        sqlx::query(
            "INSERT OR REPLACE INTO chains (chain_id, name, symbol, icon_url, threshold) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&chain.chain_id)
        .bind(&chain.name)
        .bind(&chain.symbol)
        .bind(&chain.icon_url)
        .bind(chain.threshold)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn insert_token(&self, token: &Token) -> Result<(), Error> {
        sqlx::query(
            r#"INSERT OR REPLACE INTO tokens
               (asset_id, kernel_asset_id, symbol, name, icon_url, price_btc, price_usd,
                chain_id, change_usd, change_btc, confirmations, asset_key, dust, collection_hash)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&token.asset_id)
        .bind(&token.kernel_asset_id)
        .bind(&token.symbol)
        .bind(&token.name)
        .bind(&token.icon_url)
        .bind(&token.price_btc)
        .bind(&token.price_usd)
        .bind(&token.chain_id)
        .bind(&token.change_usd)
        .bind(&token.change_btc)
        .bind(token.confirmations)
        .bind(&token.asset_key)
        .bind(&token.dust)
        .bind(&token.collection_hash)
        .execute(&self.0)
        .await?;
        Ok(())
    }
}
