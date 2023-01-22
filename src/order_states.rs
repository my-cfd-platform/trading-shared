use crate::positions::{ClosePositionReason, PositionBidAsk};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct PendingOrderState {
    pub desire_price: f64,
}

#[derive(Clone, Debug)]
pub struct ActiveOrderState {
    pub open_price: f64,
    pub open_bid_ask: PositionBidAsk,
    pub open_date: DateTime<Utc>,
    pub last_setlement_fee_date: Option<DateTime<Utc>>,
    pub next_setlement_fee_date: Option<DateTime<Utc>>,
    pub pending_order_state: Option<PendingOrderState>,
}

#[derive(Clone, Debug)]
pub struct ClosedOrderState {
    pub close_bid_ask: PositionBidAsk,
    pub close_price: f64,
    pub close_date: DateTime<Utc>,
    pub close_reason: ClosePositionReason,
    pub active_order_state: ActiveOrderState,
}
