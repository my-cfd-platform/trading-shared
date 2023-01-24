use crate::positions::{OpenedPosition, PendingPosition, Position, PositionBidAsk, PositionCalculator};
use chrono::{DateTime, Duration, Utc};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::collections::HashMap;
use uuid::Uuid;

pub struct Order {
    pub id: String,
    pub process_id: String,
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
    pub top_up_percent: Option<f64>,
    pub funding_fee_period: Option<Duration>,
    pub desired_price: Option<f64>,
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
    pub fn is_triggered(&self, profit: f64, close_price: f64, side: &OrderSide) -> bool {
        return match self.unit {
            AutoClosePositionUnit::AssetAmount => self.value >= profit,
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
    pub fn is_triggered(&self, profit: f64, close_price: f64, side: &OrderSide) -> bool {
        return match self.unit {
            AutoClosePositionUnit::AssetAmount => self.value >= profit,
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

    pub fn open(self, calculator: PositionCalculator) -> Position {
        let open_price = calculator.get_open_price(&self.instrument, &self.side);

        if let Some(desired_price) = self.desired_price {
            if open_price >= desired_price {
                return Position::Opened(self.into_opened(calculator));
            }

            return Position::Pending(self.into_pending());
        }

        Position::Opened(self.into_opened(calculator))
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

    fn into_opened(self, calculator: PositionCalculator) -> OpenedPosition {
        OpenedPosition {
            id: Position::generate_id(),
            open_price: calculator.get_open_price(&self.instrument, &self.side),
            open_date: Utc::now(),
            order: self,
            open_bidasks: calculator.take_bidasks(),
        }
    }

    fn into_pending(self) -> PendingPosition {
        PendingPosition {
            id: Position::generate_id(),
            order: self,
            create_date: Utc::now(),
        }
    }
}
