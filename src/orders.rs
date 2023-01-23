use crate::positions::{
    OpenedPosition, PendingPosition, Position, PositionBidAsk, PositionSide,
};
use chrono::{DateTime, Duration, Utc};
use std::{collections::HashMap};
use uuid::Uuid;

pub struct Order {
    pub order_id: String,
    pub process_id: String,
    pub wallet_id: String,
    pub instument: String,
    pub invest_assets: HashMap<String, f64>,
    pub leverage: f64,
    pub created_date: DateTime<Utc>,
    pub side: PositionSide,
    pub take_profit: Option<AutoClosePositionConfig>,
    pub stop_loss: Option<AutoClosePositionConfig>,
    pub stop_out_percent: f64,
    pub margin_call_percent: f64,
    pub top_up_percent: Option<f64>,
    pub funding_fee_period: Option<Duration>,
    pub desired_price: Option<f64>,
}

pub struct AutoClosePositionConfig {
    pub value: f64,
    pub unit: AutoClosePositionUnit,
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
}