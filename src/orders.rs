use crate::{
    calculations::calculate_total_amount,
    positions::{ActivePosition, BidAsk, PendingPosition, Position},
};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use rust_extensions::date_time::DateTimeAsMicroseconds;
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Order {
    pub id: String,
    pub trader_id: String,
    pub wallet_id: String,
    pub instrument: String,
    pub base_asset: String,
    pub invest_assets: HashMap<String, f64>,
    pub leverage: f64,
    pub created_date: DateTimeAsMicroseconds,
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

#[derive(Debug, PartialEq, Clone, IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub enum OrderSide {
    Buy = 0,
    Sell = 1,
}

#[derive(Debug, Clone)]
pub struct TakeProfitConfig {
    pub value: f64,
    pub unit: AutoClosePositionUnit,
}

impl TakeProfitConfig {
    pub fn is_triggered(&self, pnl: f64, close_price: f64, side: &OrderSide) -> bool {
        match self.unit {
            AutoClosePositionUnit::AssetAmount => pnl >= self.value,
            AutoClosePositionUnit::PriceRate => match side {
                OrderSide::Buy => self.value <= close_price,
                OrderSide::Sell => self.value >= close_price,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct StopLossConfig {
    pub value: f64,
    pub unit: AutoClosePositionUnit,
}

impl StopLossConfig {
    pub fn is_triggered(&self, pnl: f64, close_price: f64, side: &OrderSide) -> bool {
        match self.unit {
            AutoClosePositionUnit::AssetAmount => pnl.abs() >= self.value,
            AutoClosePositionUnit::PriceRate => match side {
                OrderSide::Buy => self.value >= close_price,
                OrderSide::Sell => self.value <= close_price,
            },
        }
    }
}

#[derive(Debug, Clone, IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub enum AutoClosePositionUnit {
    AssetAmount = 0,
    PriceRate = 1,
}

impl Order {
    pub fn get_invest_instruments(&self) -> Vec<String> {
        let mut instruments = Vec::with_capacity(self.invest_assets.len());

        for asset in self.invest_assets.keys() {
            let instrument = BidAsk::generate_id(asset, &self.base_asset);
            instruments.push(instrument);
        }

        instruments
    }

    pub fn get_instruments(&self) -> Vec<String> {
        let mut instruments = Vec::with_capacity(self.invest_assets.len() + 1);
        instruments.push(self.instrument.clone());

        for asset in self.invest_assets.keys() {
            let instrument = BidAsk::generate_id(asset, &self.base_asset);
            instruments.push(instrument);
        }

        instruments
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

    pub fn validate_prices(&self, asset_prices: &HashMap<String, f64>) -> Result<(), String> {
        for (asset, _amount) in self.invest_assets.iter() {
            let price = asset_prices.get(asset);

            if price.is_none() {
                let message = format!("Not Found price for {}", asset);
                return Err(message);
            }
        }

        Ok(())
    }

    pub fn open(self, bidask: &BidAsk, asset_prices: &HashMap<String, f64>) -> Position {
        if let Err(_) = self.validate_prices(asset_prices) {
            panic!("Can't open order: invalid prices");
        }

        if self.leverage <= 0.0 {
            panic!("Can't open order: leverage can't be less or equals zero");
        }

        let position = match self.get_type() {
            OrderType::Market => Position::Active(self.into_active(bidask, asset_prices)),
            OrderType::Limit => {
                let pending_position = self.into_pending(bidask, asset_prices);

                pending_position.try_activate()
            }
        };

        position
    }

    pub fn calculate_volume(&self, invest_amount: f64) -> f64 {
        invest_amount * self.leverage
    }

    pub fn calculate_invest_amount(&self, asset_prices: &HashMap<String, f64>) -> f64 {
        calculate_total_amount(&self.invest_assets, asset_prices)
    }

    fn into_active(self, bidask: &BidAsk, asset_prices: &HashMap<String, f64>) -> ActivePosition {
        let now = DateTimeAsMicroseconds::now();

        ActivePosition {
            id: Position::generate_id(),
            open_date: now,
            open_asset_prices: asset_prices.to_owned(),
            activate_price: bidask.get_open_price(&self.side),
            activate_date: now,
            activate_asset_prices: asset_prices.to_owned(),
            current_price: bidask.get_close_price(&self.side),
            current_asset_prices: asset_prices.to_owned(),
            last_update_date: now,
            order: self,
        }
    }

    fn into_pending(self, bidask: &BidAsk, asset_prices: &HashMap<String, f64>) -> PendingPosition {
        let now = DateTimeAsMicroseconds::now();

        PendingPosition {
            id: Position::generate_id(),
            open_date: now,
            open_asset_prices: asset_prices.to_owned(),
            current_asset_prices: asset_prices.to_owned(),
            current_price: bidask.get_open_price(&self.side),
            last_update_date: now,
            order: self,
        }
    }
}
