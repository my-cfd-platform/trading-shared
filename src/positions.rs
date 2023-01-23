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
pub enum OrderSide {
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
    pub fn get_close_price(&self, side: &OrderSide) -> f64 {
        match side {
            OrderSide::Buy => self.bid,
            OrderSide::Sell => self.ask,
        }
    }

    pub fn get_open_price(&self, side: &OrderSide) -> f64 {
        match side {
            OrderSide::Buy => self.ask,
            OrderSide::Sell => self.bid,
        }
    }
}

pub enum Position {
    Opened(OpenedPosition),
    Closed(ClosedPosition),
    Pending(PendingPosition),
}

impl Position {
    pub fn get_id(&self) -> &str {
        match self {
            Position::Opened(position) => &position.id,
            Position::Closed(position) => &position.id,
            Position::Pending(position) => &position.id,
        }
    }

    pub fn get_order(&self) -> &Order {
        match self {
            Position::Opened(position) => &position.order,
            Position::Closed(position) => &position.order,
            Position::Pending(position) => &position.order,
        }
    }
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
    pub fn close(self, bidask: PositionBidAsk, reason: ClosePositionReason) -> ClosedPosition {
        return ClosedPosition {
            close_date: Utc::now(),
            close_price: bidask.get_close_price(&self.order.side),
            close_reason: reason,
            id: self.id.clone(),
            close_bid_ask: bidask,
            order: self.order,
            open_bid_ask: self.open_bid_ask,
            open_date: self.open_date,
            open_price: self.open_price,
            profit: self.profit,
        };
    }

    pub fn calculate_profit(&self, invest_amount: f64, close_price: f64) -> f64 {
        let volume = self.order.calculate_volume(invest_amount);

        let profit = match self.order.side {
            OrderSide::Buy => (close_price / self.open_price - 1.0) * volume,
            OrderSide::Sell => (close_price / self.open_price - 1.0) * -volume,
        };

        profit
    }

    pub fn is_stop_out(&self, invest_amount: f64, profit: f64) -> bool {
        let margin_percent = self.calculate_margin_percent(invest_amount, profit);

        100.0 - margin_percent >= self.order.stop_out_percent
    }

    pub fn is_margin_call(&self, invest_amount: f64, profit: f64) -> bool {
        let margin_percent = self.calculate_margin_percent(invest_amount, profit);

        100.0 - margin_percent >= self.order.margin_call_percent
    }

    fn calculate_margin_percent(&self, invest_amount: f64, profit: f64) -> f64 {
        let margin = profit + invest_amount;
        let margin_percent = margin / invest_amount * 100.0;

        margin_percent
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
