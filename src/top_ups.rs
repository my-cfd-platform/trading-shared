use std::collections::HashMap;
use rust_extensions::date_time::DateTimeAsMicroseconds;

#[derive(Debug, Clone)]
pub struct ActiveTopUp {
    pub id: String,
    pub date: DateTimeAsMicroseconds,
    pub assets: HashMap<String, f64>,
    pub instrument_price: f64,
    pub asset_prices: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct CanceledTopUp {
    pub id: String,
    pub date: DateTimeAsMicroseconds,
    pub assets: HashMap<String, f64>,
    pub instrument_price: f64,
    pub asset_prices: HashMap<String, f64>,
    pub cancel_instrument_price: f64,
    pub cancel_date: DateTimeAsMicroseconds,
}
