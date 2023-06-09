use ahash::HashMap;
use rust_extensions::date_time::DateTimeAsMicroseconds;

#[derive(Debug, Clone)]
pub struct TopUp {
    pub id: String,
    pub date: DateTimeAsMicroseconds,
    pub assets: HashMap<String, f64>,
    pub instrument_price: f64,
    pub asset_prices: HashMap<String, f64>,
}
