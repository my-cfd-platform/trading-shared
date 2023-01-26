use crate::{
    calculations::get_open_price,
    positions::{ActivePosition, BidAsk, PendingPosition, Position},
};
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

    pub fn open(self, bidasks: &HashMap<String, BidAsk>) -> Position {
        let open_price = get_open_price(bidasks, &self.instrument, &self.side);

        if let Some(desired_price) = self.desire_price {
            if open_price >= desired_price {
                return Position::Active(self.into_active(bidasks));
            }

            return Position::Pending(self.into_pending(bidasks));
        }

        Position::Active(self.into_active(bidasks))
    }

    pub fn calculate_volume(&self, invest_amount: f64) -> f64 {
        invest_amount * self.leverage
    }

    pub fn calculate_invest_amount(&self, bidasks_by_instruments: &HashMap<String, BidAsk>) -> f64 {
        let mut amount = 0.0;

        for (invest_asset, invest_amount) in self.invest_assets.iter() {
            // todo: generate by instrument model
            let instrument = format!("{}{}", invest_asset, self.base_asset);
            let bidask = bidasks_by_instruments
                .get(&instrument)
                .expect(&format!("BidAsk not found for {}", self.instrument));
            let asset_amount = bidask.ask * invest_amount;
            amount += asset_amount;
        }

        amount
    }

    pub fn calculate_invest_amounts(
        &self,
        bidasks: &HashMap<String, BidAsk>,
    ) -> HashMap<String, f64> {
        let mut amounts = HashMap::with_capacity(self.invest_assets.len());

        for (invest_asset, invest_amount) in self.invest_assets.iter() {
            let instrument = BidAsk::generate_id(invest_asset, &self.base_asset);
            let bidask = bidasks
                .get(&instrument)
                .expect(&format!("BidAsk not found for {}", instrument));
            let estimated_amount = bidask.ask * invest_amount;
            amounts.insert(invest_asset.to_owned(), estimated_amount);
        }

        amounts
    }

    fn into_active(self, bidasks: &HashMap<String, BidAsk>) -> ActivePosition {
        let now = Utc::now();
        let invest_amounts = self.calculate_invest_amounts(bidasks);

        ActivePosition {
            id: Position::generate_id(),
            open_date: now,
            open_invest_amounts: invest_amounts.clone(),
            activate_price: get_open_price(bidasks, &self.instrument, &self.side),
            activate_date: now,
            activate_invest_amounts: invest_amounts,
            order: self,
        }
    }

    fn into_pending(self, bidasks: &HashMap<String, BidAsk>) -> PendingPosition {
        PendingPosition {
            id: Position::generate_id(),
            open_date: Utc::now(),
            open_invest_amounts: self.calculate_invest_amounts(bidasks),
            order: self,
        }
    }
}
