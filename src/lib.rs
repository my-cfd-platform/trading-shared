pub mod positions;
pub mod orders;
pub mod caches;
pub mod calculations;
pub mod monitoring;
pub mod top_ups;
pub mod wallets;
pub mod instrument_symbol;
pub mod position_id;
pub mod asset_symbol;
pub mod wallet_id;
pub mod assets;
pub mod sharding;

pub use ahash::AHashMap;

#[cfg(test)]
mod tests {
    use crate::positions::BidAsk;

    #[test]
    fn test_get_instrument_symbol() {
        let instrument_symbol = BidAsk::get_instrument_symbol(&"BTC".into(), &"USD".into());

        assert_eq!(instrument_symbol, "BTCUSD".into());
    }
}
