use crate::orders::{Order, OrderCalculator, OrderSide};
use chrono::{DateTime, Utc};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use uuid::Uuid;

#[derive(Clone, IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub enum ClosePositionReason {
    ClientCommand = 0,
    StopOut = 1,
    TakeProfit = 2,
    StopLoss = 3,
    AdminCommand = 4,
}

#[derive(Clone)]
pub struct BidAsk {
    pub instrument: String,
    pub datetime: DateTime<Utc>,
    pub bid: f64,
    pub ask: f64,
}

impl BidAsk {
    pub fn generate_id(base_asset: &str, quote_asset: &str) -> String {
        let id = format!("{}{}", base_asset, quote_asset); // todo: find better solution

        id
    }

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
    Active(ActivePosition),
    Closed(ClosedPosition),
    Pending(PendingPosition),
}

impl Position {
    pub fn generate_id() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn get_id(&self) -> &str {
        match self {
            Position::Active(position) => &position.id,
            Position::Closed(position) => &position.id,
            Position::Pending(position) => &position.id,
        }
    }

    pub fn get_create_invest_amount(&self) -> f64 {
        match self {
            Position::Active(position) => position.open_invest_amount,
            Position::Closed(position) => position.create_invest_amount,
            Position::Pending(position) => position.open_invest_amount,
        }
    }

    pub fn get_create_date(&self) -> DateTime<Utc> {
        match self {
            Position::Active(position) => position.open_date,
            Position::Closed(position) => position.create_date,
            Position::Pending(position) => position.open_date,
        }
    }

    pub fn get_order(&self) -> &Order {
        match self {
            Position::Active(position) => &position.order,
            Position::Closed(position) => &position.order,
            Position::Pending(position) => &position.order,
        }
    }

    pub fn get_order_mut(&mut self) -> &mut Order {
        match self {
            Position::Active(position) => &mut position.order,
            Position::Closed(position) => &mut position.order,
            Position::Pending(position) => &mut position.order,
        }
    }
}

pub struct PendingPosition {
    pub id: String,
    pub order: Order,
    pub open_date: DateTime<Utc>,
    pub open_invest_amount: f64,
    pub open_bidasks: Vec<BidAsk>,
}

pub struct ActivePosition {
    pub id: String,
    pub order: Order,
    pub open_date: DateTime<Utc>,
    pub open_invest_amount: f64,
    pub open_bidasks: Vec<BidAsk>,
    pub activate_price: f64,
    pub activate_date: DateTime<Utc>,
    pub activate_invest_amount: f64,
}

impl ActivePosition {
    pub fn close(self, calculator: OrderCalculator, reason: ClosePositionReason) -> ClosedPosition {
        if !calculator.can_calculate(&self.order) {
            panic!("Invalid calculator for position")
        }

        let invested_amount =
            calculator.calculate_invest_amount(&self.order.invest_assets, &self.order.base_asset);
        let close_price = calculator.get_close_price(&self.order.instrument, &self.order.side);

        return ClosedPosition {
            id: self.id.clone(),
            create_date: self.open_date,
            create_invest_amount: self.open_invest_amount,
            open_date: self.activate_date,
            open_price: self.activate_price,
            open_invest_amount: self.activate_invest_amount,
            close_date: Utc::now(),
            close_price,
            close_reason: reason,
            close_bidasks: calculator.take_bidasks(),
            close_invest_amount: invested_amount,
            pnl: self.calculate_pnl(invested_amount, close_price),
            order: self.order,
        };
    }

    pub fn calculate_pnl(&self, invest_amount: f64, close_price: f64) -> f64 {
        let volume = self.order.calculate_volume(invest_amount);

        let pnl = match self.order.side {
            OrderSide::Buy => (close_price / self.activate_price - 1.0) * volume,
            OrderSide::Sell => (close_price / self.activate_price - 1.0) * -volume,
        };

        pnl
    }

    pub fn is_stop_out(&self, invest_amount: f64, pnl: f64) -> bool {
        let margin_percent = self.calculate_margin_percent(invest_amount, pnl);

        100.0 - margin_percent >= self.order.stop_out_percent
    }

    pub fn is_margin_call(&self, invest_amount: f64, pnl: f64) -> bool {
        let margin_percent = self.calculate_margin_percent(invest_amount, pnl);

        100.0 - margin_percent >= self.order.margin_call_percent
    }

    fn calculate_margin_percent(&self, invest_amount: f64, pnl: f64) -> f64 {
        let margin = pnl + invest_amount;
        let margin_percent = margin / invest_amount * 100.0;

        margin_percent
    }
}

pub struct ClosedPosition {
    pub id: String,
    pub create_date: DateTime<Utc>,
    pub create_invest_amount: f64,
    pub order: Order,
    pub open_price: f64,
    pub open_date: DateTime<Utc>,
    pub open_invest_amount: f64,
    pub close_price: f64,
    pub close_date: DateTime<Utc>,
    pub close_reason: ClosePositionReason,
    pub close_invest_amount: f64,
    pub pnl: f64,
    pub close_bidasks: Vec<BidAsk>,
}
