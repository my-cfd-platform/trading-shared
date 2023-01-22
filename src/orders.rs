use std::collections::HashMap;
use chrono::{DateTime, Duration, Utc};
use crate::{positions::{PositionSide, PositionBidAsk, Position}, order_states::ActiveOrderState};

#[derive(Debug, Clone)]
pub struct Order {
    pub id: String,
    pub process_id: String,
    pub wallet_id: String,
    pub instument: String,
    pub invested_assets: HashMap<String, f64>,
    pub leverage: f64,
    pub create_date: DateTime<Utc>,
    pub side: PositionSide,
    pub take_profit: Option<AutoClosePositionConfig>,
    pub stop_loss: Option<AutoClosePositionConfig>,
    pub last_update_date: DateTime<Utc>,
    pub stop_out_percent: f64,
    pub margin_call_percent: f64,
    pub top_up_percent: f64,
    pub top_up_enabled: f64,
    pub setlement_fee_period: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct AutoClosePositionConfig {
    pub value: f64,
    pub unit: AutoClosePositionUnit,
}

#[derive(Debug, Clone)]
#[repr(i32)]
pub enum AutoClosePositionUnit {
    AssetAmount,
    PriceRate,
}

impl Order {
    pub fn open_position(self, bidask: PositionBidAsk) -> Position {    
        let state = ActiveOrderState {
            open_price: bidask.get_open_price(self.side.clone()),
            open_bid_ask: bidask,
            open_date: Utc::now(),
            pending_order_state: None,
            // todo: set settelment fee
            last_setlement_fee_date: None,
            next_setlement_fee_date: None,
        };

        Position::Active(state, self)
    }
}
