use crate::{
    calculations::calculate_margin_percent,
    orders::{Order, OrderSide, StopLossConfig, TakeProfitConfig},
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

    pub fn get_asset_price(&self, asset: &str) -> f64 {
        if self.instrument.starts_with(asset) {
            self.ask
        } else {
            0.0
        }
    }
}

#[derive(Clone)]
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

    pub fn get_status(&self) -> PositionStatus {
        match self {
            Position::Pending(_position) => PositionStatus::Pending,
            Position::Active(_position) => PositionStatus::Pending,
            Position::Closed(position) => {
                if position.activate_date.is_some() {
                    PositionStatus::Filled
                } else {
                    PositionStatus::Canceled
                }
            }
        }
    }
}

#[derive(Clone, IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub enum PositionStatus {
    Pending = 0,
    Active = 1,
    Filled = 2,
    Canceled = 3,
}

#[derive(Clone)]
pub struct PendingPosition {
    pub id: String,
    pub order: Order,
    pub open_date: DateTime<Utc>,
    pub open_asset_prices: HashMap<String, f64>,
}

impl PendingPosition {
    pub fn try_activate(self, price: f64, asset_prices: &HashMap<String, f64>) -> Position {
        self.order.validate_prices(asset_prices);

        if let Some(desired_price) = self.order.desire_price {
            if price >= desired_price && self.order.side == OrderSide::Sell {
                return Position::Active(self.into_active(price, asset_prices));
            }

            if price <= desired_price && self.order.side == OrderSide::Buy {
                return Position::Active(self.into_active(price, asset_prices));
            }

            return Position::Pending(self);
        } else {
            panic!("PendingPosition without desire price");
        }
    }

    pub fn set_take_profit(&mut self, value: Option<TakeProfitConfig>) {
        self.order.take_profit = value;
    }

    pub fn set_stop_loss(&mut self, value: Option<StopLossConfig>) {
        self.order.stop_loss = value;
    }

    pub fn set_desire_price(&mut self, value: f64) {
        self.order.desire_price = Some(value);
    }

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

    fn into_active(self, price: f64, asset_prices: &HashMap<String, f64>) -> ActivePosition {
        ActivePosition {
            id: self.id,
            open_date: self.open_date,
            open_asset_prices: self.open_asset_prices,
            activate_price: price,
            activate_date: Utc::now(),
            activate_asset_prices: asset_prices.to_owned(),
            order: self.order,
        }
    }
}

#[derive(Clone)]
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
    pub fn set_take_profit(&mut self, value: Option<TakeProfitConfig>) {
        self.order.take_profit = value;
    }

    pub fn set_stop_loss(&mut self, value: Option<StopLossConfig>) {
        self.order.stop_loss = value;
    }

    pub fn close(
        self,
        close_price: f64,
        asset_prices: &HashMap<String, f64>,
        reason: ClosePositionReason,
    ) -> ClosedPosition {
        let invest_amount = self.order.calculate_invest_amount(asset_prices);

        return ClosedPosition {
            pnl: Some(self.calculate_pnl(invest_amount, close_price)),
            asset_pnls: self.calculate_asset_pnls(close_price),
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

    pub fn try_close(self, close_price: f64, asset_prices: &HashMap<String, f64>) -> Position {
        if self.is_stop_out(asset_prices, close_price) {
            return Position::Closed(self.close(
                close_price,
                asset_prices,
                ClosePositionReason::StopOut,
            ));
        }

        if self.is_stop_loss(asset_prices, close_price) {
            return Position::Closed(self.close(
                close_price,
                asset_prices,
                ClosePositionReason::StopLoss,
            ));
        }

        if self.is_take_profit(asset_prices, close_price) {
            return Position::Closed(self.close(
                close_price,
                asset_prices,
                ClosePositionReason::TakeProfit,
            ));
        }

        Position::Active(self)
    }

    fn is_take_profit(&self, asset_prices: &HashMap<String, f64>, close_price: f64) -> bool {
        if let Some(take_profit_config) = self.order.take_profit.as_ref() {
            let invest_amount = self.order.calculate_invest_amount(asset_prices);
            let pnl = self.calculate_pnl(invest_amount, close_price);

            take_profit_config.is_triggered(pnl, close_price, &self.order.side)
        } else {
            false
        }
    }

    fn is_stop_loss(&self, asset_prices: &HashMap<String, f64>, close_price: f64) -> bool {
        if let Some(stop_loss_config) = self.order.stop_loss.as_ref() {
            let invest_amount = self.order.calculate_invest_amount(asset_prices);
            let pnl = self.calculate_pnl(invest_amount, close_price);

            stop_loss_config.is_triggered(pnl, close_price, &self.order.side)
        } else {
            false
        }
    }

    fn is_stop_out(&self, asset_prices: &HashMap<String, f64>, close_price: f64) -> bool {
        let invest_amount = self.order.calculate_invest_amount(asset_prices);
        let pnl = self.calculate_pnl(invest_amount, close_price);
        let margin_percent = calculate_margin_percent(invest_amount, pnl);

        100.0 - margin_percent >= self.order.stop_out_percent
    }

    pub fn is_margin_call(&self, asset_prices: &HashMap<String, f64>, close_price: f64) -> bool {
        let invest_amount = self.order.calculate_invest_amount(asset_prices);
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

    fn calculate_asset_pnls(&self, close_price: f64) -> HashMap<String, f64> {
        let mut pnls_by_assets = HashMap::with_capacity(self.order.invest_assets.len());

        for (asset, amount) in self.order.invest_assets.iter() {
            let pnl = self.calculate_pnl(*amount, close_price);
            pnls_by_assets.insert(asset.to_owned(), pnl);
        }

        pnls_by_assets
    }
}

#[derive(Clone)]
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;

    use crate::{orders::Order, positions::Position};

    use super::ClosePositionReason;

    #[tokio::test]
    async fn close_active_position() {
        let order = Order {
            base_asset: "USDT".to_string(),
            id: "test".to_string(),
            instrument: "ATOMUSDT".to_string(),
            trader_id: "test".to_string(),
            wallet_id: "test".to_string(),
            created_date: Utc::now(),
            desire_price: None,
            funding_fee_period: None,
            invest_assets: HashMap::from([("BTC".to_string(), 100.0)]),
            leverage: 1.0,
            side: crate::orders::OrderSide::Buy,
            take_profit: None,
            stop_loss: None,
            stop_out_percent: 10.0,
            margin_call_percent: 10.0,
            top_up_enabled: false,
            top_up_percent: 10.0,
        };
        let prices = HashMap::from([("BTC".to_string(), 22300.0)]);
        let position = order.open(14.748, &prices);
        let position = match position {
            Position::Active(position) => position,
            _ => {
                panic!("Invalid position")
            }
        };

        let closed_position = position.close(14.75, &prices, ClosePositionReason::ClientCommand);

        let pnl = closed_position.pnl.unwrap();
        let asset_pnl = *closed_position.asset_pnls.get("BTC").unwrap();
        assert_ne!(pnl, asset_pnl);
        assert_eq!(302.41388662883173, pnl);
        assert_eq!(0.01356116083537362, asset_pnl);
    }
}
