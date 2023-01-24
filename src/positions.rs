use std::collections::HashMap;

use crate::orders::{Order, OrderSide};
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
    Canceled = 4,
    AdminAction = 5,
    InsufficientCollateral = 6,
}

#[derive(Clone)]
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
    pub fn generate_id() -> String {
        Uuid::new_v4().to_string()
    }

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
    pub create_date: DateTime<Utc>,
}

pub struct OpenedPosition {
    pub id: String,
    pub order: Order,
    pub open_price: f64,
    pub open_date: DateTime<Utc>,
    pub open_bidasks: Vec<PositionBidAsk>,
}

impl OpenedPosition {
    pub fn close(
        self,
        calculator: PositionCalculator,
        reason: ClosePositionReason,
    ) -> ClosedPosition {
        let position = Position::Opened(self);

        if !calculator.can_calculate(&position) {
            panic!("Invalid calculator for position")
        }

        let position = match position {
            Position::Opened(position) => position,
            _ => panic!("Imposible")
        };
        let invested_amount = calculator.calculate_invest_amount(&position.order.invest_assets, &position.order.base_asset);
        let close_price = calculator.get_close_price(&position.order.instrument, &position.order.side);

        return ClosedPosition {
            close_date: Utc::now(),
            close_price: close_price,
            close_reason: reason,
            id: position.id.clone(),
            open_date: position.open_date,
            open_price: position.open_price,
            profit: position.calculate_profit(invested_amount, close_price),
            order: position.order,
            close_bidasks: calculator.take_bidasks(),
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
    pub open_date: DateTime<Utc>,
    pub close_price: f64,
    pub close_date: DateTime<Utc>,
    pub close_reason: ClosePositionReason,
    pub profit: f64,
    pub close_bidasks: Vec<PositionBidAsk>,
}

pub struct PositionCalculator {
    position_id: String,
    bidasks: HashMap<String, PositionBidAsk>,
}

impl PositionCalculator {
    pub fn new(position: &Position, bidasks: HashMap<String, PositionBidAsk>) -> Self {
        let order = position.get_order();

        if let None = bidasks.get(&order.instrument) {
            panic!("BidAsk not found for {}", order.instrument);
        }

        for (asset, _amount) in order.invest_assets.iter() {
            // todo: generate by instrument model
            let instrument = format!("{}{}", asset, order.base_asset);
            let _bidask = bidasks
                .get(&instrument)
                .expect(&format!("BidAsk not found for {}", instrument));
        }

        Self {
            bidasks,
            position_id: position.get_id().to_owned(),
        }
    }

    pub fn can_calculate(&self, position: &Position) -> bool {
        if self.position_id != position.get_id() {
            return false;
        }

        return true;
    }

    pub fn get_average_price(&self, instrument: &str) -> f64 {
        let bidask = self
            .bidasks
            .get(instrument)
            .expect(&format!("BidAsk not found for {}", instrument));
        let price = (bidask.bid + bidask.ask) / 2.0;

        return price;
    }

    pub fn get_close_price(&self, instrument: &str, side: &OrderSide) -> f64 {
        let bidask = self
            .bidasks
            .get(instrument)
            .expect(&format!("BidAsk not found for {}", instrument));
        let price = bidask.get_close_price(side);

        return price;
    }

    pub fn get_open_price(&self, instrument: &str, side: &OrderSide) -> f64 {
        let bidask = self
            .bidasks
            .get(instrument)
            .expect(&format!("BidAsk not found for {}", instrument));
        let price = bidask.get_open_price(side);

        return price;
    }

    pub fn calculate_invest_amount(&self, invest_assets: &HashMap<String, f64>, base_asset: &str) -> f64 {
        let mut amount = 0.0;

        for (invest_asset, invest_amount) in invest_assets.iter() {
            // todo: generate by instrument model
            let instrument = format!("{}{}", invest_asset, base_asset);
            let bidask = self
                .bidasks
                .get(&instrument)
                .expect(&format!("BidAsk not found for {}", instrument));
            let asset_price = bidask.ask + bidask.bid / 2.0;
            let asset_amount = asset_price * invest_amount;
            amount += asset_amount;
        }

        amount
    }

    pub fn take_bidasks(self) -> Vec<PositionBidAsk> {
        self.bidasks.into_values().collect()
    }
}
