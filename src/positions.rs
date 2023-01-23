use crate::orders::Order;
use chrono::{DateTime, Utc};

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

#[repr(i32)]
pub enum PositionSide {
    Buy = 0,
    Sell = 1,
}

pub struct PositionBidAsk {
    pub instrument: String,
    pub datetime: DateTime<Utc>,
    pub bid: f64,
    pub ask: f64,
}

impl PositionBidAsk {
    pub fn get_close_price(&self, side: &PositionSide) -> f64 {
        match side {
            PositionSide::Buy => self.bid,
            PositionSide::Sell => self.ask,
        }
    }

    pub fn get_open_price(&self, side: &PositionSide) -> f64 {
        match side {
            PositionSide::Buy => self.ask,
            PositionSide::Sell => self.bid,
        }
    }
}

pub enum Position {
    Opened(OpenedPosition),
    Closed(ClosedPosition),
    Pending(PendingPosition),
}

pub struct PendingPosition {
    pub id: String,
    pub order: Order,
}

pub struct OpenedPosition {
    pub id: String,
    pub order: Order,
    pub open_price: f64,
    pub open_bid_ask: PositionBidAsk,
    pub open_date: DateTime<Utc>,
    pub last_setlement_fee_date: Option<DateTime<Utc>>,
    pub next_setlement_fee_date: Option<DateTime<Utc>>,
    pub profit: f64,
}

impl OpenedPosition {
    pub fn close(self, bid_ask: PositionBidAsk, reason: ClosePositionReason) -> ClosedPosition {
        return ClosedPosition {
            close_date: Utc::now(),
            close_price: bid_ask.get_close_price(&self.order.side),
            close_reason: reason,
            id: self.id.clone(),
            close_bid_ask: bid_ask,
            order: self.order,
            open_bid_ask: self.open_bid_ask,
            open_date: self.open_date,
            open_price: self.open_price,
            profit: self.profit,
        };
    }
}

pub struct ClosedPosition {
    pub id: String,
    pub order: Order,
    pub open_price: f64,
    pub open_bid_ask: PositionBidAsk,
    pub open_date: DateTime<Utc>,
    pub close_bid_ask: PositionBidAsk,
    pub close_price: f64,
    pub close_date: DateTime<Utc>,
    pub close_reason: ClosePositionReason,
    pub profit: f64,
}
