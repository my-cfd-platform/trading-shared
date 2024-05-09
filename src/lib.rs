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
mod assets;


pub use ahash::AHashMap;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
