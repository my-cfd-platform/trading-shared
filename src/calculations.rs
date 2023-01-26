use std::collections::HashMap;
use crate::{orders::OrderSide, positions::BidAsk};

pub fn get_close_price(
    bidasks: &HashMap<String, BidAsk>,
    instrument: &str,
    side: &OrderSide,
) -> f64 {
    let bidask = bidasks
        .get(instrument)
        .expect(&format!("BidAsk not found for {}", instrument));
    let price = bidask.get_close_price(side);

    return price;
}

pub fn get_open_price(
    bidasks: &HashMap<String, BidAsk>,
    instrument: &str,
    side: &OrderSide,
) -> f64 {
    let bidask = bidasks
        .get(instrument)
        .expect(&format!("BidAsk not found for {}", instrument));
    let price = bidask.get_open_price(side);

    return price;
}
