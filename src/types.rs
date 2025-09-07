use ethers::types::{Address, H256, U256};

#[derive(Clone, Debug)]
pub struct Transfer {
    pub block_number: u64,
    pub tx_hash: H256,
    pub log_index: u64,
    pub token: Address,
    pub from: Address,
    pub to: Address,
    pub amount_raw: U256,
    pub amount: f64,          
    pub timestamp: u64,
}
