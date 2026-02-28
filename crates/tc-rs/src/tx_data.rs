use alloy_primitives::{Address, U256};

#[derive(Debug, Clone)]
pub struct TxData {
    pub to: Address,
    pub data: Vec<u8>,
    pub value: U256,
}

impl TxData {
    pub fn new(to: Address, data: Vec<u8>, value: U256) -> Self {
        TxData { to, data, value }
    }
}
