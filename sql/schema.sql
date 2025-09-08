PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = on;


-- Raw chain objects we actually persist for traceability
CREATE TABLE IF NOT EXISTS blocks(
	number INTEGER PRIMARY KEY,
	hash BLOB NOT NULL,
	parent_hash BLOB NOT NULL,
	time_stamp INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS erc20_transfers(
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	block_number INTEGER NOT NULL,
	tx_hash BLOB NOT NULL,
	log_index INTEGER NOT NULL,
	token_address BLOB NOT NULL,
	from_address BLOB NOT NULL,
	to_address BLOB NOT NULL,
	amount_raw TEXT NOT NULL,
	amount REAL NOT NULL,
	occured_at INTEGER NOT NULL,
	UNIQUE(tx_hash, log_index)
);

-- Exchanges & addresses

CREATE TABLE IF NOT EXISTS exchanges (
  id            INTEGER PRIMARY KEY,
  name          TEXT UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS exchange_addresses (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  exchange_id   INTEGER NOT NULL REFERENCES exchanges(id) ON DELETE CASCADE,
  address       BLOB NOT NULL,
  UNIQUE(exchange_id, address)
);


-- Rolling aggregates
CREATE TABLE IF NOT EXISTS net_flows (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  exchange_id     INTEGER NOT NULL REFERENCES exchanges(id) ON DELETE CASCADE,
  token_address   BLOB NOT NULL,
  token_symbol    TEXT NOT NULL,
  token_decimals  INTEGER NOT NULL,
  cumulative_in   REAL NOT NULL DEFAULT 0,
  cumulative_out  REAL NOT NULL DEFAULT 0,
  cumulative_net  REAL NOT NULL DEFAULT 0,
  last_updated_bn INTEGER NOT NULL DEFAULT 0,
  UNIQUE(exchange_id, token_address)
);


-- Indexing checkpoints
CREATE TABLE IF NOT EXISTS checkpoints (
  id              INTEGER PRIMARY KEY CHECK (id = 1),
  last_block_seen INTEGER NOT NULL
);

INSERT OR IGNORE INTO exchanges (id, name) VALUES (1, 'binance');
INSERT OR IGNORE INTO checkpoints (id, last_block_seen) VALUES (1, 0);

-- Helpful indexes
CREATE INDEX IF NOT EXISTS idx_transfers_token ON erc20_transfers(token_address);
CREATE INDEX IF NOT EXISTS idx_transfers_time  ON erc20_transfers(occurred_at);
CREATE INDEX IF NOT EXISTS idx_transfers_part  ON erc20_transfers(from_address, to_address);