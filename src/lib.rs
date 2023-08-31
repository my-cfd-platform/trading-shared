pub mod positions;
pub mod orders;
pub mod caches;
pub mod calculations;
pub mod monitoring;
pub mod top_ups;
pub mod wallets;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
