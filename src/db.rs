use crate::types::Transfer;
use anyhow::Result;
use ethers::types::Address;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

#[derive(Clone)]
pub struct Db(pub Pool<Sqlite>);

impl Db {
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(url)
            .await?;
        Ok(Self(pool))
    }

    pub async fn migrate(&self) -> Result<()> {
        let sql = include_str!("../sql/schema.sql");
        sqlx::query(sql).execute(&self.0).await?;
        Ok(())
    }

    pub async fn upsert_exchange_addresses(&self, exchange_id: i64, addrs: &[Address]) -> Result<()> {
        let mut tx = self.0.begin().await?;
        for a in addrs {
            sqlx::query!("INSERT OR IGNORE INTO exchange_addresses (exchange_id, address) VALUES (?, ?)", 
                         exchange_id, a.as_bytes())
                .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn ensure_netflow_row(&self, exchange_id: i64, token: Address, symbol: &str, decimals: u8) -> Result<()> {
        sqlx::query!(
            r#"INSERT OR IGNORE INTO net_flows (exchange_id, token_address, token_symbol, token_decimals)
               VALUES (?, ?, ?, ?)"#,
            exchange_id,
            token.as_bytes(),
            symbol,
            decimals as i64
        ).execute(&self.0).await?;
        Ok(())
    }

    pub async fn insert_transfer_and_update_netflow(&self, exchange_id: i64, t: &Transfer, is_inflow: bool) -> Result<()> {
        let mut tx = self.0.begin().await?;

        sqlx::query!(
            r#"INSERT OR IGNORE INTO blocks (number, hash, parent_hash, timestamp) VALUES (?, ?, ?, ?)"#,
            t.block_number as i64,
            t.tx_hash.as_bytes(),
            &[][..],                  // parent not strictly needed here for MVP
            t.timestamp as i64
        ).execute(&mut *tx).await?;

        sqlx::query!(
            r#"INSERT OR IGNORE INTO erc20_transfers
               (block_number, tx_hash, log_index, token_address, from_address, to_address, amount_raw, amount, occurred_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            t.block_number as i64,
            t.tx_hash.as_bytes(),
            t.log_index as i64,
            t.token.as_bytes(),
            t.from.as_bytes(),
            t.to.as_bytes(),
            t.amount_raw.to_string(),
            t.amount,
            t.timestamp as i64
        ).execute(&mut *tx).await?;

        // Update rolling totals
        if is_inflow {
            sqlx::query!(
                r#"UPDATE net_flows
                   SET cumulative_in = cumulative_in + ?, 
                       cumulative_net = cumulative_in + ? - cumulative_out,
                       last_updated_bn = ?
                   WHERE exchange_id = ? AND token_address = ?"#,
                t.amount,
                t.amount,
                t.block_number as i64,
                exchange_id,
                t.token.as_bytes()
            ).execute(&mut *tx).await?;
        } else {
            sqlx::query!(
                r#"UPDATE net_flows
                   SET cumulative_out = cumulative_out + ?, 
                       cumulative_net = cumulative_in - (cumulative_out + ?),
                       last_updated_bn = ?
                   WHERE exchange_id = ? AND token_address = ?"#,
                t.amount,
                t.amount,
                t.block_number as i64,
                exchange_id,
                t.token.as_bytes()
            ).execute(&mut *tx).await?;
        }

        // Checkpoint
        sqlx::query!(
            r#"UPDATE checkpoints SET last_block_seen = MAX(last_block_seen, ?)"#,
            t.block_number as i64
        ).execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn fetch_netflow(&self, exchange_id: i64, token: Address) -> Result<Option<(f64,f64,f64,i64)>> {
        let row = sqlx::query!(
            r#"SELECT cumulative_in, cumulative_out, cumulative_net, last_updated_bn 
               FROM net_flows WHERE exchange_id = ? AND token_address = ?"#,
            exchange_id,
            token.as_bytes()
        ).fetch_optional(&self.0).await?;
        Ok(row.map(|r| (r.cumulative_in, r.cumulative_out, r.cumulative_net, r.last_updated_bn)))
    }
}