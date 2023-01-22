use crate::{
    orders::Order,
    position_states::{ActiveOrderState, ClosedOrderState, PendingOrderState},
};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
#[repr(i32)]
pub enum ClosePositionReason {
    None = 0,
    ClientCommand = 1,
    StopOut = 2,
    TakeProfit = 3,
    StopLoss = 4,
    Canceled = 5,
    AdminAction = 6,
    InsufficientCollateral = 7,
}

#[derive(Debug, Clone)]
#[repr(i32)]
pub enum PositionSide {
    Buy = 0,
    Sell = 1,
}

#[derive(Clone, Debug)]
pub struct PositionBidAsk {
    pub instrument: String,
    pub datetime: DateTime<Utc>,
    pub bid: f64,
    pub ask: f64,
}

impl PositionBidAsk {
    pub fn get_close_price(&self, side: PositionSide) -> f64 {
        match side {
            PositionSide::Buy => self.bid,
            PositionSide::Sell => self.ask,
        }
    }

    pub fn get_open_price(&self, side: PositionSide) -> f64 {
        match side {
            PositionSide::Buy => self.ask,
            PositionSide::Sell => self.bid,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Position {
    Active(ActiveOrderState, Order),
    Closed(ClosedOrderState, Order),
    Pending(PendingOrderState, Order),
}
