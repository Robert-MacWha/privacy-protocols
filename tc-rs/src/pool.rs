use alloy::primitives::Address;

pub enum Asset {
    Native {
        symbol: String,
        decimals: u8,
    },
    Erc20 {
        address: Address,
        symbol: String,
        decimals: u8,
    },
}

pub trait Pool {
    fn chain_id(&self) -> u64;
    fn address(&self) -> Address;
    fn asset(&self) -> Asset;
    fn amount_wei(&self) -> u128;

    fn display(&self) -> String {
        format!(
            "eip155:{}/{}/{}",
            self.chain_id(),
            self.symbol(),
            self.amount(),
        )
    }

    fn symbol(&self) -> String {
        match self.asset() {
            Asset::Native { symbol, .. } => symbol.clone(),
            Asset::Erc20 { symbol, .. } => symbol.clone(),
        }
    }

    /// Decimal amount as a string, e.g. "0.1"
    fn amount(&self) -> String {
        let decimals = match self.asset() {
            Asset::Native { decimals, .. } => decimals,
            Asset::Erc20 { decimals, .. } => decimals,
        };

        format_amount(self.amount_wei(), decimals)
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
