use ahash::AHashMap;

pub struct Wallet {
    pub id: String,
    pub balances_by_asset: AHashMap<String, WalletBalance>,
}

pub struct WalletBalance {
    pub id: String,
    pub asset_symbol: String,
    pub available_asset_amount: f64,
}