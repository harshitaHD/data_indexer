use anyhow::{Context, Result};
use ethers::types::Address;
use std::{env, str::FromStr};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub ws_url: String,
    pub http_url: String,
    pub db_url: String,
    pub pol_token: Address,
    pub pol_symbol: String,
    pub pol_decimals: Option<u8>,
    pub binance_addrs: Vec<Address>,
    pub http_listen_addr: String,
}

fn parse_addresses(csv: &str) -> Result<Vec<Address>> {
    csv.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| Address::from_str(s).context(format!("invalid address: {s}")))
        .collect()
}

pub fn load() -> Result<AppConfig> {
    dotenvy::dotenv().ok();
    Ok(AppConfig {
        ws_url: env::var("POLYGON_RPC_WS").context("POLYGON_RPC_WS not set")?,
        http_url: env::var("POLYGON_RPC_HTTP").context("POLYGON_RPC_HTTP not set")?,
        db_url: env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://netflow.db".into()),
        pol_token: Address::from_str(&env::var("POL_TOKEN_ADDRESS")?).context("POL_TOKEN_ADDRESS invalid")?,
        pol_symbol: env::var("POL_TOKEN_SYMBOL").unwrap_or_else(|_| "POL".into()),
        pol_decimals: env::var("POL_TOKEN_DECIMALS").ok().and_then(|v| v.parse::<u8>().ok()),
        binance_addrs: parse_addresses(&env::var("BINANCE_ADDRESSES").context("BINANCE_ADDRESSES not set")?)?,
        http_listen_addr: env::var("HTTP_LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
    })
}
