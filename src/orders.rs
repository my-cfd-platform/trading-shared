use crate::positions::{ActivePosition, PendingPosition, Position, PositionBidAsk};
use chrono::{DateTime, Duration, Utc};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::collections::HashMap;
use uuid::Uuid;

pub struct Order {
    pub id: String,
    pub wallet_id: String,
    pub instrument: String,
    pub base_asset: String,
    pub invest_assets: HashMap<String, f64>,
    pub leverage: f64,
    pub created_date: DateTime<Utc>,
    pub side: OrderSide,
    pub take_profit: Option<TakeProfitConfig>,
    pub stop_loss: Option<StopLossConfig>,
    pub stop_out_percent: f64,
    pub margin_call_percent: f64,
    pub top_up_enabled: bool,
    pub top_up_percent: f64,
    pub funding_fee_period: Option<Duration>,
    pub desire_price: Option<f64>,
}

#[derive(Clone, IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub enum OrderSide {
    Buy = 0,
    Sell = 1,
}
#[derive(Clone)]
pub struct TakeProfitConfig {
    pub value: f64,
    pub unit: AutoClosePositionUnit,
}

impl TakeProfitConfig {
    pub fn is_triggered(&self, pnl: f64, close_price: f64, side: &OrderSide) -> bool {
        return match self.unit {
            AutoClosePositionUnit::AssetAmount => self.value >= pnl,
            AutoClosePositionUnit::PriceRate => match side {
                OrderSide::Buy => {
                    return self.value <= close_price;
                }
                OrderSide::Sell => {
                    return self.value >= close_price;
                }
            },
        };
    }
}

#[derive(Clone)]
pub struct StopLossConfig {
    pub value: f64,
    pub unit: AutoClosePositionUnit,
}

impl StopLossConfig {
    pub fn is_triggered(&self, pnl: f64, close_price: f64, side: &OrderSide) -> bool {
        return match self.unit {
            AutoClosePositionUnit::AssetAmount => self.value >= pnl,
            AutoClosePositionUnit::PriceRate => match side {
                OrderSide::Buy => {
                    return self.value >= close_price;
                }
                OrderSide::Sell => {
                    return self.value <= close_price;
                }
            },
        };
    }
}

#[derive(Clone, IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub enum AutoClosePositionUnit {
    AssetAmount,
    PriceRate,
}

impl Order {
    pub fn generate_id() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn open(self, calculator: OrderCalculator) -> Position {
        let open_price = calculator.get_open_price(&self.instrument, &self.side);

        if let Some(desired_price) = self.desire_price {
            if open_price >= desired_price {
                return Position::Active(self.into_opened(calculator));
            }

            return Position::Pending(self.into_pending(calculator));
        }

        Position::Active(self.into_opened(calculator))
    }

    pub fn calculate_volume(&self, invest_amount: f64) -> f64 {
        invest_amount * self.leverage
    }

    pub fn calculate_invest_amount(
        &self,
        bidasks_by_instruments: &HashMap<String, PositionBidAsk>,
    ) -> f64 {
        let mut amount = 0.0;

        for (invest_asset, invest_amount) in self.invest_assets.iter() {
            // todo: generate by instrument model
            let instrument = format!("{}{}", invest_asset, self.base_asset);
            let bidask = bidasks_by_instruments
                .get(&instrument)
                .expect(&format!("BidAsk not found for {}", self.instrument));
            let asset_price = bidask.ask + bidask.bid / 2.0;
            let asset_amount = asset_price * invest_amount;
            amount += asset_amount;
        }

        amount
    }

    fn into_opened(self, calculator: OrderCalculator) -> ActivePosition {
        let now = Utc::now();
        let invest_amount = calculator.calculate_invest_amount(&self.invest_assets, &self.base_asset);

        ActivePosition {
            id: Position::generate_id(),
            open_date: now,
            open_invest_amount: invest_amount,
            activate_price: calculator.get_open_price(&self.instrument, &self.side),
            activate_date: now,
            activate_invest_amount: invest_amount,
            order: self,
            open_bidasks: calculator.take_bidasks(),
        }
    }

    fn into_pending(self, calculator: OrderCalculator) -> PendingPosition {
        PendingPosition {
            id: Position::generate_id(),
            open_date: Utc::now(),
            open_invest_amount: calculator.calculate_invest_amount(&self.invest_assets, &self.base_asset),
            order: self,
            open_bidasks: calculator.take_bidasks(),
        }
    }
}

pub struct OrderCalculator {
    order_id: String,
    bidasks: HashMap<String, PositionBidAsk>,
}

impl OrderCalculator {
    pub fn new(order: &Order, bidasks: HashMap<String, PositionBidAsk>) -> Self {

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
            order_id: order.id.clone(),
        }
    }

    pub fn can_calculate(&self, order: &Order) -> bool {
        if self.order_id != order.id {
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

    pub fn calculate_invest_amount(
        &self,
        invest_assets: &HashMap<String, f64>,
        base_asset: &str,
    ) -> f64 {
        let mut amount = 0.0;

        for (invest_asset, invest_amount) in invest_assets.iter() {
            let instrument = PositionBidAsk::generate_id(invest_asset, base_asset);
            let bidask = self
                .bidasks
                .get(&instrument)
                .expect(&format!("BidAsk not found for {}", instrument));
            let estimated_amount = bidask.ask * invest_amount;
            amount += estimated_amount;
        }

        amount
    }

    pub fn take_bidasks(self) -> Vec<PositionBidAsk> {
        self.bidasks.into_values().collect()
    }
}
