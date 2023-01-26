use std::collections::HashMap;

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

    pub fn get_open_invest_amounts(&self) -> &HashMap<String, f64> {
        match self {
            Position::Active(position) => &position.open_invest_amounts,
            Position::Closed(position) => &position.open_invest_amounts,
            Position::Pending(position) => &position.open_invest_amounts,
        }
    }

    pub fn get_open_date(&self) -> DateTime<Utc> {
        match self {
            Position::Active(position) => position.open_date,
            Position::Closed(position) => position.open_date,
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
    pub open_invest_amounts: HashMap<String, f64>,
}

pub struct ActivePosition {
    pub id: String,
    pub order: Order,
    pub open_date: DateTime<Utc>,
    pub open_invest_amounts: HashMap<String, f64>,
    pub activate_price: f64,
    pub activate_date: DateTime<Utc>,
    pub activate_invest_amounts: HashMap<String, f64>,
}

impl ActivePosition {
    pub fn close(self, calculator: OrderCalculator, reason: ClosePositionReason) -> ClosedPosition {
        if !calculator.can_calculate(&self.order) {
            panic!("Invalid calculator for position")
        }

        let invest_amounts =
            calculator.calculate_invest_amounts(&self.order.invest_assets, &self.order.base_asset);
        let close_price = calculator.get_close_price(&self.order.instrument, &self.order.side);
        let total_invest_amount = invest_amounts.values().sum();

        return ClosedPosition {
            id: self.id.clone(),
            pnl: self.calculate_pnl(total_invest_amount, close_price),
            open_date: self.open_date,
            open_invest_amounts: self.open_invest_amounts,
            activate_date: self.activate_date,
            activate_price: self.activate_price,
            activate_invest_amounts: self.activate_invest_amounts,
            close_date: Utc::now(),
            close_price,
            close_reason: reason,
            close_invest_amounts: invest_amounts,
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

    pub fn calculate_asset_pnls(
        &self,
        invest_amounts: HashMap<String, f64>,
        close_price: f64,
    ) -> HashMap<String, f64> {
        let mut pnls_by_assets = HashMap::with_capacity(self.order.invest_assets.len());
        let total_investment = invest_amounts.values().sum();
        let pnl = self.calculate_pnl(total_investment, close_price);

        for (asset, amount) in invest_amounts {
            let percent = amount / total_investment * 100.0;
            let asset_pnl = pnl * percent / 100.0;
            pnls_by_assets.insert(asset, asset_pnl);
        }

        pnls_by_assets
    }

    pub fn is_stop_out(&self, invest_amount: f64, close_price: f64) -> bool {
        let pnl = self.calculate_pnl(invest_amount, close_price);
        let margin_percent = self.calculate_margin_percent(invest_amount, pnl);

        100.0 - margin_percent >= self.order.stop_out_percent
    }

    pub fn is_margin_call(&self, invest_amount: f64, close_price: f64) -> bool {
        let pnl = self.calculate_pnl(invest_amount, close_price);
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
    pub order: Order,
    pub open_date: DateTime<Utc>,
    pub open_invest_amounts: HashMap<String, f64>,
    pub activate_price: f64,
    pub activate_date: DateTime<Utc>,
    pub activate_invest_amounts: HashMap<String, f64>,
    pub close_price: f64,
    pub close_date: DateTime<Utc>,
    pub close_reason: ClosePositionReason,
    pub close_invest_amounts: HashMap<String, f64>,
    pub pnl: f64,
}
