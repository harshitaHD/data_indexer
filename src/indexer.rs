use crate::{config::AppConfig, db::Db, types::Transfer};
use anyhow::{Context, Result};
use ethers::{
    abi::AbiDecode,
    prelude::*,
    providers::{Middleware, Provider, Ws},
    types::{Filter, H256, Log, Address, BlockNumber, U256},
};
use once_cell::sync::Lazy;
use std::{collections::HashSet, sync::Arc};

static TRANSFER_TOPIC: Lazy<H256> = Lazy::new(|| H256::from_slice(keccak256("Transfer(address,address,uint256)").as_slice()));

pub struct Indexer {
    cfg: AppConfig,
    db: Db,
    ws: Arc<Provider<Ws>>,
    http: Arc<Provider<Http>>,
    exchange_id: i64,
    decimals: u8,
    binance_set: HashSet<Address>,
}

impl Indexer {
    pub async fn new(cfg: AppConfig, db: Db) -> Result<Self> {
        let ws = Provider::<Ws>::connect(cfg.ws_url.clone()).await?;
        let http = Provider::<Http>::try_from(cfg.http_url.clone())?;

        // Determine token decimals if not set
        let decimals = if let Some(d) = cfg.pol_decimals {
            d
        } else {
            // Minimal ERC20 decimals() call
            let sig = ethers::utils::id("decimals()")[0..4].to_vec();
            let call = &ethers::types::TransactionRequest::default()
                .to(cfg.pol_token)
                .data(sig);
            let bytes = http.call(call.clone(), None).await?;
            let d = U256::from_big_endian(&bytes[bytes.len().saturating_sub(32)..]).as_u32() as u8;
            d
        };

        // Ensure DB rows
        db.ensure_netflow_row(1, cfg.pol_token, &cfg.pol_symbol, decimals).await?;
        db.upsert_exchange_addresses(1, &cfg.binance_addrs).await?;

        Ok(Self {
            binance_set: cfg.binance_addrs.iter().cloned().collect(),
            cfg, db, ws: Arc::new(ws), http: Arc::new(http),
            exchange_id: 1, decimals
        })
    }

    pub async fn run(self) -> Result<()> {
        // Subscribe to Transfer logs for POL, with OR filter on from/to (indexed topics)
        // topics: [Transfer, from?, to?]
        let from_topics: Vec<H256> = self.binance_set.iter().map(|a| H256::from_slice(a.as_fixed_bytes())).collect();
        let to_topics   = from_topics.clone();

        let filter = Filter::new()
            .address(self.cfg.pol_token)
            .event(&*TRANSFER_TOPIC)
            .topic1(ValueOrArray::Array(from_topics.clone()))
            .topic2(ValueOrArray::Array(to_topics.clone()));

        // We also want cases where from is ANY and to is Binance, or from is Binance and to is ANY.
        // ethers Filter doesn't support union of two filters in one; so we subscribe to all Transfer logs for POL
        // and apply address checks in-process (cheap).
        let broad_filter = Filter::new()
            .address(self.cfg.pol_token)
            .topic0(*TRANSFER_TOPIC)
            .from_block(BlockNumber::Latest); // live only, no backfill

        let mut stream = self.ws.subscribe_logs(&broad_filter).await?;

        while let Some(log) = stream.next().await {
            if let Err(e) = self.process_log(log).await {
                tracing::error!(error=%e, "failed processing log");
            }
        }
        Ok(())
    }

    async fn process_log(&self, log: Log) -> Result<()> {
        // Decode topics: Transfer(address indexed from, address indexed to, uint256 value)
        if log.topics.len() != 3 { return Ok(()); }
        let from = Address::from_slice(&log.topics[1].as_bytes()[12..]);
        let to   = Address::from_slice(&log.topics[2].as_bytes()[12..]);

        // Only keep events where either side is a Binance address
        let is_in = self.binance_set.contains(&to);
        let is_out = self.binance_set.contains(&from);
        if !(is_in || is_out) { return Ok(()); }

        // amount is in data
        let amount_raw = U256::from_big_endian(log.data.get(0..32).unwrap_or(&[0u8;32]));
        let amount = normalize(amount_raw, self.decimals);

        // Fetch the block timestamp via HTTP (cheap)
        let bn = log.block_number.context("missing block number")?.as_u64();
        let block = self.http.get_block(bn).await?.context("block not found")?;
        let ts = block.timestamp.as_u64();

        let t = Transfer {
            block_number: bn,
            tx_hash: log.transaction_hash.unwrap_or_default(),
            log_index: log.log_index.unwrap_or_default().as_u64(),
            token: self.cfg.pol_token,
            from, to,
            amount_raw,
            amount,
            timestamp: ts,
        };

        self.db.insert_transfer_and_update_netflow(self.exchange_id, &t, is_in).await?;
        Ok(())
    }
}

fn normalize(v: U256, decimals: u8) -> f64 {
    // Convert U256 into f64 by dividing by 10^decimals (safe enough for dashboarding; raw is stored too)
    let scale = 10u128.pow(decimals as u32) as f64;
    let hi = (v >> 64).as_u128();
    let lo = (v & U256::from(u128::MAX)).as_u128();
    let as_f = (hi as f64) * (2f64.powi(64)) + (lo as f64);
    as_f / scale
}
