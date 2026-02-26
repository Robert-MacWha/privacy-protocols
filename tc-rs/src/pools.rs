use alloy::primitives::{Address, address};

use crate::pool::{Asset, Pool};

#[derive(Copy, Clone)]
pub struct Eth1Pool {}

impl Pool for Eth1Pool {
    fn chain_id(&self) -> u64 {
        1
    }
    fn address(&self) -> Address {
        address!("0x8cc930096b4df705a007c4a039bdfa1320ed2508")
    }
    fn asset(&self) -> Asset {
        Asset::Native {
            symbol: "ETH".to_string(),
            decimals: 18,
        }
    }
    fn amount_wei(&self) -> u128 {
        10_u128.pow(18)
    }
}
