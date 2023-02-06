use crate::positions::{ActivePosition, PendingPosition, Position, BidAsk};
use chrono::{DateTime, Duration, Utc};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct Order {
    pub id: String,
    pub trader_id: String,
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
pub enum OrderType {
    Market = 0,
    Limit = 1,
}

#[derive(PartialEq, Clone, IntoPrimitive, TryFromPrimitive)]
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

    pub fn get_invest_instruments(&self) -> Vec<String> {
        let mut instuments = Vec::with_capacity(self.invest_assets.len());

        for asset in self.invest_assets.keys() {
            let instrument = BidAsk::generate_id(asset, &self.base_asset);
            instuments.push(instrument);
        }

        instuments
    }

    pub fn get_type(&self) -> OrderType {
        if self.desire_price.is_some() {
            OrderType::Limit
        } else {
            OrderType::Market
        }
    }

    pub fn generate_id() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn validate_prices(&self, asset_prices: &HashMap<String, f64>) {
        for (asset, _amount) in self.invest_assets.iter() {
            let price = asset_prices.get(asset);

            if price.is_none() {
                panic!("Can't open order. No price for {}", asset);
            }
        }
    }

    pub fn open(self, price: f64, asset_prices: &HashMap<String, f64>) -> Position {
        self.validate_prices(asset_prices);

        let position = match self.get_type() {
            OrderType::Market => Position::Active(self.into_active(price, asset_prices)),
            OrderType::Limit => {
                let pending_position = self.into_pending(price, asset_prices);

                pending_position.try_activate()
            }
        };

        position
    }

    pub fn calculate_volume(&self, invest_amount: f64) -> f64 {
        invest_amount * self.leverage
    }

    pub fn calculate_invest_amount(&self, asset_prices: &HashMap<String, f64>) -> f64 {
        let mut total_amount = 0.0;

        for (asset, amount) in self.invest_assets.iter() {
            let price = asset_prices
                .get(asset)
                .expect(&format!("Price not found for {}", asset));
            let estimated_amount = price * amount;
            total_amount += estimated_amount;
        }

        total_amount
    }

    fn into_active(self, price: f64, asset_prices: &HashMap<String, f64>) -> ActivePosition {
        let now = Utc::now();

        ActivePosition {
            id: Position::generate_id(),
            open_date: now,
            open_asset_prices: asset_prices.to_owned(),
            activate_price: price,
            activate_date: now,
            activate_asset_prices: asset_prices.to_owned(),
            order: self,
            current_price: price,
            current_asset_prices: asset_prices.to_owned(),
            last_update_date: Utc::now(),
        }
    }

    fn into_pending(self, price: f64, asset_prices: &HashMap<String, f64>) -> PendingPosition {
        PendingPosition {
            id: Position::generate_id(),
            open_date: Utc::now(),
            open_asset_prices: asset_prices.to_owned(),
            order: self,
            current_asset_prices: asset_prices.to_owned(),
            current_price: price,
            last_update_date: Utc::now(),
        }
    }
}
