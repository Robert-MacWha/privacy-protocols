use std::fmt::Display;

use alloy_primitives::{Address, address};
use serde::{Deserialize, Serialize};

use crate::note::Note;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Asset {
    Native {
        symbol: &'static str,
        decimals: u8,
    },
    Erc20 {
        address: Address,
        symbol: &'static str,
        decimals: u8,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Pool {
    pub chain_id: u64,
    pub address: Address,
    pub asset: Asset,
    pub amount_wei: u128,
}

pub const POOLS: &[Pool] = &[SEPOLIA_ETHER_1, ETHEREUM_ETHER_100];

pub const SEPOLIA_ETHER_1: Pool = Pool {
    chain_id: 11155111,
    address: address!("0x8cc930096b4df705a007c4a039bdfa1320ed2508"),
    asset: Asset::Native {
        symbol: "ETH",
        decimals: 18,
    },
    amount_wei: 1 * 10_u128.pow(18),
};

pub const ETHEREUM_ETHER_100: Pool = Pool {
    chain_id: 1,
    address: address!("0xA160cdAB225685dA1d56aa342Ad8841c3b53f291"),
    asset: Asset::Native {
        symbol: "ETH",
        decimals: 18,
    },
    amount_wei: 100 * 10_u128.pow(18),
};

impl Pool {
    pub fn from_note(note: &Note) -> Option<Self> {
        Self::from_id(&note.amount, &note.symbol, note.chain_id)
    }

    pub fn from_id(amount: &str, symbol: &str, chain_id: u64) -> Option<Self> {
        POOLS
            .iter()
            .find(|pool| {
                pool.chain_id == chain_id && pool.symbol() == symbol && pool.amount() == amount
            })
            .cloned()
    }

    pub fn symbol(&self) -> String {
        match &self.asset {
            Asset::Native { symbol, .. } => symbol.to_string(),
            Asset::Erc20 { symbol, .. } => symbol.to_string(),
        }
    }

    /// Decimal amount as a string, e.g. "0.1"
    pub fn amount(&self) -> String {
        let decimals = match &self.asset {
            Asset::Native { decimals, .. } => *decimals,
            Asset::Erc20 { decimals, .. } => *decimals,
        };

        format_amount(self.amount_wei, decimals)
    }
}

impl Display for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "eip155:{}/{}/{}",
            self.chain_id,
            self.symbol(),
            self.amount()
        )
    }
}

fn format_amount(amount: u128, decimals: u8) -> String {
    if decimals == 0 {
        return amount.to_string();
    }

    let decimals = decimals as usize;
    let divisor = 10u128.pow(decimals as u32);

    let whole = amount / divisor;
    let frac = amount % divisor;

    if frac == 0 {
        return whole.to_string();
    }

    // Pad fractional part with leading zeros
    let mut frac_str = format!("{:0width$}", frac, width = decimals);

    // Trim trailing zeros
    while frac_str.ends_with('0') {
        frac_str.pop();
    }

    format!("{whole}.{frac_str}")
}
