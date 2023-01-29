use crate::{
    calculations::{calculate_margin_percent},
    orders::{Order, OrderSide},
};
use chrono::{DateTime, Utc};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::collections::HashMap;
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

    pub fn get_open_asset_prices(&self) -> &HashMap<String, f64> {
        match self {
            Position::Active(position) => &position.open_asset_prices,
            Position::Closed(position) => &position.open_asset_prices,
            Position::Pending(position) => &position.open_asset_prices,
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
    pub open_asset_prices: HashMap<String, f64>,
}

impl PendingPosition {
    pub fn close(
        self,
        close_price: f64,
        asset_prices: &HashMap<String, f64>,
        reason: ClosePositionReason,
    ) -> ClosedPosition {
        return ClosedPosition {
            pnl: None,
            asset_pnls: HashMap::new(),
            open_date: self.open_date,
            open_asset_prices: self.open_asset_prices,
            activate_date: None,
            activate_price: None,
            activate_asset_prices: HashMap::new(),
            close_date: Utc::now(),
            close_price,
            close_reason: reason,
            close_asset_prices: asset_prices.to_owned(),
            order: self.order,
            id: self.id,
        };
    }
}

pub struct ActivePosition {
    pub id: String,
    pub order: Order,
    pub open_date: DateTime<Utc>,
    pub open_asset_prices: HashMap<String, f64>,
    pub activate_price: f64,
    pub activate_date: DateTime<Utc>,
    pub activate_asset_prices: HashMap<String, f64>,
}

impl ActivePosition {
    pub fn close(
        self,
        close_price: f64,
        asset_prices: &HashMap<String, f64>,
        reason: ClosePositionReason,
    ) -> ClosedPosition {
        let invest_amount = self.order.calculate_invest_amount(asset_prices);

        return ClosedPosition {
            pnl: Some(self.calculate_pnl(invest_amount, close_price)),
            asset_pnls: self.calculate_asset_pnls(invest_amount, &asset_prices, close_price),
            open_date: self.open_date,
            open_asset_prices: self.open_asset_prices,
            activate_date: Some(self.activate_date),
            activate_price: Some(self.activate_price),
            activate_asset_prices: self.activate_asset_prices,
            close_date: Utc::now(),
            close_price,
            close_reason: reason,
            close_asset_prices: asset_prices.to_owned(),
            order: self.order,
            id: self.id,
        };
    }

    pub fn is_stop_out(&self, invest_amount: f64, close_price: f64) -> bool {
        let pnl = self.calculate_pnl(invest_amount, close_price);
        let margin_percent = calculate_margin_percent(invest_amount, pnl);

        100.0 - margin_percent >= self.order.stop_out_percent
    }

    pub fn is_margin_call(&self, invest_amount: f64, close_price: f64) -> bool {
        let pnl = self.calculate_pnl(invest_amount, close_price);
        let margin_percent = calculate_margin_percent(invest_amount, pnl);

        100.0 - margin_percent >= self.order.margin_call_percent
    }

    fn calculate_pnl(&self, invest_amount: f64, close_price: f64) -> f64 {
        let volume = self.order.calculate_volume(invest_amount);

        let pnl = match self.order.side {
            OrderSide::Buy => (close_price / self.activate_price - 1.0) * volume,
            OrderSide::Sell => (close_price / self.activate_price - 1.0) * -volume,
        };

        pnl
    }

    fn calculate_asset_pnls(
        &self,
        invest_amount: f64,
        asset_prices: &HashMap<String, f64>,
        close_price: f64,
    ) -> HashMap<String, f64> {
        let mut pnls_by_assets = HashMap::with_capacity(self.order.invest_assets.len());
        let pnl = self.calculate_pnl(invest_amount, close_price);

        for (asset, amount) in self.order.invest_assets.iter() {
            let percent = amount / invest_amount * 100.0;
            let pnl_amount_in_base_asset = pnl * percent / 100.0;
            let asset_price = asset_prices.get(asset).expect("Failed to get asset price");
            let pnl_amount_in_asset = pnl_amount_in_base_asset * asset_price;
            pnls_by_assets.insert(asset.to_owned(), pnl_amount_in_asset);
        }

        pnls_by_assets
    }
}

pub struct ClosedPosition {
    pub id: String,
    pub order: Order,
    pub open_date: DateTime<Utc>,
    pub open_asset_prices: HashMap<String, f64>,
    pub activate_price: Option<f64>,
    pub activate_date: Option<DateTime<Utc>>,
    pub activate_asset_prices: HashMap<String, f64>,
    pub close_price: f64,
    pub close_date: DateTime<Utc>,
    pub close_reason: ClosePositionReason,
    pub close_asset_prices: HashMap<String, f64>,
    pub pnl: Option<f64>,
    pub asset_pnls: HashMap<String, f64>,
}
