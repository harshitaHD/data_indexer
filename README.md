# Polygon Net-Flow Indexer (POL → Binance)

Real-time indexer for POL ERC-20 transfers on Polygon, computing cumulative net flows to Binance (inflow - outflow). No backfill. Built in Rust; stores raw and processed data in SQLite; ships with an HTTP API and CLI.

## Tech
- Rust (tokio, ethers-rs, sqlx, axum)
- SQLite (WAL mode)
- Polygon RPC (WS + HTTP)

## Setup
1. `cp .env.example .env` and edit with real endpoints and POL contract address.
2. `cargo build --release`
3. `./target/release/polygon-netflow-indexer run`
4. `curl http://localhost:8080/netflow` or `./target/release/polygon-netflow-indexer netflow`

## Schema
See `sql/schema.sql`. Raw transfers recorded in `erc20_transfers`; rolling totals in `net_flows`.

## Extending to multiple exchanges
- Add new `exchanges` rows and addresses in `exchange_addresses`.
- The indexer can maintain multiple exchange HashSets and update `net_flows` per event.

## Notes
- Floating `amount` is for display; exact value is stored as `amount_raw` (decimal U256).
- No historical backfill in this phase; starts at `latest` head.
