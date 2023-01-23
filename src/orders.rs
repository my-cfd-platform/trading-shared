use crate::{
    caches::BidAsksCache,
    positions::{OpenedPosition, OrderSide, PendingPosition, Position, PositionBidAsk},
};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use uuid::Uuid;

pub struct Order {
    pub order_id: String,
    pub process_id: String,
    pub wallet_id: String,
    pub instument: String,
    pub base_assets: String,
    pub invest_assets: HashMap<String, f64>,
    pub leverage: f64,
    pub created_date: DateTime<Utc>,
    pub side: OrderSide,
    pub take_profit: Option<TakeProfitConfig>,
    pub stop_loss: Option<StopLossConfig>,
    pub stop_out_percent: f64,
    pub margin_call_percent: f64,
    pub top_up_percent: Option<f64>,
    pub funding_fee_period: Option<Duration>,
    pub desired_price: Option<f64>,
}

pub struct TakeProfitConfig {
    pub value: f64,
    pub unit: AutoClosePositionUnit,
}

impl TakeProfitConfig {
    pub fn is_triggered(&self, profit: f64, close_price: f64, side: &OrderSide) -> bool {
        return match self.unit {
            AutoClosePositionUnit::AssetAmount => self.value >= profit,
            AutoClosePositionUnit::PriceRate => match side {
                OrderSide::Buy => {
                    return self.value <= close_price;
                }
                OrderSide::Sell => {
                    return self.value >= close_price;
                }
            },
        };
    }
}

pub struct StopLossConfig {
    pub value: f64,
    pub unit: AutoClosePositionUnit,
}

impl StopLossConfig {
    pub fn is_triggered(&self, profit: f64, close_price: f64, side: &OrderSide) -> bool {
        return match self.unit {
            AutoClosePositionUnit::AssetAmount => self.value >= profit,
            AutoClosePositionUnit::PriceRate => match side {
                OrderSide::Buy => {
                    return self.value >= close_price;
                }
                OrderSide::Sell => {
                    return self.value <= close_price;
                }
            },
        };
    }
}

#[repr(i32)]
pub enum AutoClosePositionUnit {
    AssetAmount,
    PriceRate,
}

impl Order {
    pub fn open(self, bidask: PositionBidAsk) -> Position {
        let id = Uuid::new_v4().to_string();

        if let Some(_desired_price) = self.desired_price {
            return Position::Pending(PendingPosition {
                id: id,
                order: self,
            });
        }

        let position = OpenedPosition {
            id: id,
            open_price: bidask.get_open_price(&self.side),
            open_bid_ask: bidask,
            open_date: Utc::now(),
            order: self,
            profit: 0.0,
            // todo: set settelment fee
            last_setlement_fee_date: None,
            next_setlement_fee_date: None,
        };

        Position::Opened(position)
    }

    pub fn calculate_invest_amount(&self, prices: &BidAsksCache) -> f64 {
        let mut amount = 0.0;

        for (invest_asset, invest_amount) in self.invest_assets.iter() {
            let instrument = format!("{}{}", invest_asset, self.base_assets); // todo: generate by instrument model
            let asset_price = prices.get_average_price(&instrument);

            if let Some(asset_price) = asset_price {
                let asset_amount = asset_price * invest_amount;
                amount += asset_amount;
            }
            else  {
                panic!("Not found price for instrument {}", self.instument);
            }
        }

        amount
    }

    pub fn calculate_volume(&self, invest_amount: f64) -> f64 {
        invest_amount * self.leverage
    }
}
