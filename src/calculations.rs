use crate::{orders::OrderSide, positions::BidAsk};
use std::collections::HashMap;

pub fn get_close_price(
    bidasks: &HashMap<String, BidAsk>,
    instrument: &str,
    side: &OrderSide,
) -> f64 {
    let bidask = bidasks
        .get(instrument)
        .unwrap_or_else(|| panic!("BidAsk not found for {}", instrument));

    bidask.get_close_price(side)
}

pub fn get_open_price(
    bidasks: &HashMap<String, BidAsk>,
    instrument: &str,
    side: &OrderSide,
) -> f64 {
    let bidask = bidasks
        .get(instrument)
        .unwrap_or_else(|| panic!("BidAsk not found for {}", instrument));

    bidask.get_open_price(side)
}

pub fn calculate_margin_percent(invest_amount: f64, pnl: f64) -> f64 {
    let margin = pnl + invest_amount;

    margin / invest_amount * 100.0
}

pub fn calculate_percent(from_number: f64, number: f64) -> f64 {
    number / from_number * 100.0
}

pub fn calculate_total_amount(
    asset_amounts: &HashMap<String, f64>,
    asset_prices: &HashMap<String, f64>,
) -> f64 {
    let mut total_amount = 0.0;

    for (asset, amount) in asset_amounts.iter() {
        let price = asset_prices
            .get(asset)
            .unwrap_or_else(|| panic!("Price not found for {}", asset));
        let estimated_amount = price * amount;
        total_amount += estimated_amount;
    }

    total_amount
}

pub fn ceil(x: f64, precision: u32) -> f64 {
    let y = 10_i64.pow(precision) as f64;
    (x * y).ceil() / y
}

pub fn floor(x: f64, precision: u32) -> f64 {
    let y = 10_i64.pow(precision) as f64;
    (x * y).floor() / y
}

pub fn round(x: f64, precision: u32) -> f64 {
    let y = 10_i64.pow(precision) as f64;
    (x * y).round() / y
}
