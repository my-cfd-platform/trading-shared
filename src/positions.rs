use crate::calculations::{calculate_percent, floor};
use crate::top_ups::{ActiveTopUp, CanceledTopUp};
use crate::{
    calculations::calculate_total_amount,
    orders::{Order, OrderSide, StopLossConfig, TakeProfitConfig},
};
use ahash::{HashSet, HashSetExt};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use rust_extensions::date_time::DateTimeAsMicroseconds;
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, IntoPrimitive, TryFromPrimitive)]
#[repr(i32)]
pub enum ClosePositionReason {
    ClientCommand = 0,
    StopOut = 1,
    TakeProfit = 2,
    StopLoss = 3,
    AdminCommand = 4,
    InsufficientBalance = 5,
}

#[derive(Clone)]
pub struct BidAsk {
    pub instrument: String,
    pub datetime: DateTimeAsMicroseconds,
    pub bid: f64,
    pub ask: f64,
}

impl BidAsk {
    pub fn new_synthetic(id: impl Into<String>, bid: f64, ask: f64) -> Self {
        Self {
            instrument: id.into(),
            datetime: DateTimeAsMicroseconds::now(),
            bid,
            ask,
        }
    }

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

    pub fn get_asset_price(&self, asset: &str, side: &OrderSide) -> f64 {
        match side {
            OrderSide::Sell => {
                if self.instrument.starts_with(asset) {
                    self.ask
                } else {
                    panic!("Invalid instrument {} for asset {}", self.instrument, asset)
                }
            }
            OrderSide::Buy => {
                if self.instrument.starts_with(asset) {
                    self.bid
                } else {
                    panic!("Invalid instrument {} for asset {}", self.instrument, asset)
                }
            }
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

    pub fn get_open_date(&self) -> DateTimeAsMicroseconds {
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
            Position::Active(_position) => PositionStatus::Active,
            Position::Closed(position) => position.get_status(),
        }
    }

    pub fn get_instruments(&self) -> HashSet<String> {
        match self {
            Position::Pending(position) => position.order.get_instruments().into_iter().collect(),
            Position::Active(position) => {
                let order_instruments = position.order.get_instruments();
                let mut instruments = self.get_top_up_instruments(&position.top_ups);
                instruments.extend(order_instruments.into_iter());

                instruments
            }
            Position::Closed(position) => {
                let order_instruments = position.order.get_instruments();
                let mut instruments = self.get_top_up_instruments(&position.top_ups);
                instruments.extend(order_instruments.into_iter());

                instruments
            }
        }
    }

    fn get_top_up_instruments(&self, top_ups: &Vec<ActiveTopUp>) -> HashSet<String> {
        let mut instruments = HashSet::new();

        for top_up in top_ups {
            for (asset_symbol, _asset_amount) in top_up.assets.iter() {
                let instrument = format!("{}{}", asset_symbol, self.get_order().base_asset);
                instruments.insert(instrument);
            }
        }

        instruments
    }
}

#[derive(Clone, IntoPrimitive, TryFromPrimitive, PartialEq)]
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
    pub open_price: f64,
    pub open_date: DateTimeAsMicroseconds,
    pub open_asset_prices: HashMap<String, f64>,
    pub current_price: f64,
    pub current_asset_prices: HashMap<String, f64>,
    pub last_update_date: DateTimeAsMicroseconds,
    pub total_invest_assets: HashMap<String, f64>,
}

impl PendingPosition {
    pub fn update(&mut self, bidask: &BidAsk) {
        self.try_update_instrument_price(bidask);
        self.try_update_asset_prices(bidask);
        self.last_update_date = DateTimeAsMicroseconds::now();
    }

    pub fn can_activate(&self) -> bool {
        let Some(desired_price) = self.order.desire_price else {
            panic!("PendingPosition without desire price");
        };

        let is_limit_sell = self.order.side == OrderSide::Sell && self.open_price <= desired_price;

        if is_limit_sell && self.current_price >= desired_price {
            return true;
        }

        let is_limit_buy = self.order.side == OrderSide::Buy && self.open_price >= desired_price;

        if is_limit_buy && self.current_price <= desired_price {
            return true;
        }

        let is_stop_sell = self.order.side == OrderSide::Sell && self.open_price >= desired_price;

        if is_stop_sell && self.current_price >= desired_price {
            return true;
        }

        let is_stop_buy = self.order.side == OrderSide::Buy && self.open_price <= desired_price;

        if is_stop_buy && self.current_price <= desired_price {
            return true;
        }

        false
    }

    fn try_update_instrument_price(&mut self, bidask: &BidAsk) {
        if self.order.instrument == bidask.instrument {
            self.current_price = bidask.get_open_price(&self.order.side)
        }
    }

    fn try_update_asset_prices(&mut self, bidask: &BidAsk) {
        for asset in self.order.invest_assets.keys() {
            let id = BidAsk::generate_id(asset, &self.order.base_asset);

            if id == bidask.instrument {
                let price = bidask.get_asset_price(asset, &OrderSide::Sell);
                let current_asset_price = self.current_asset_prices.get_mut(asset);

                if let Some(current_asset_price) = current_asset_price {
                    *current_asset_price = price;
                } else {
                    self.current_asset_prices.insert(asset.to_owned(), price);
                }
            }
        }
    }

    pub fn try_activate(self) -> Position {
        if self.can_activate() {
            return Position::Active(self.into_active());
        }

        Position::Pending(self)
    }

    pub fn into_active(self) -> ActivePosition {
        if !self.can_activate() {
            panic!("Can't activate");
        }

        let now = DateTimeAsMicroseconds::now();

        ActivePosition {
            id: self.id,
            open_price: self.open_price,
            open_date: self.open_date,
            open_asset_prices: self.open_asset_prices,
            activate_price: self.current_price,
            activate_date: now,
            activate_asset_prices: self.current_asset_prices.to_owned(),
            order: self.order,
            current_price: self.current_price,
            current_asset_prices: self.current_asset_prices,
            last_update_date: now,
            top_ups: Vec::new(),
            current_pnl: 0.0,
            current_loss_percent: 0.0,
            prev_loss_percent: 0.0,
            top_up_locked: false,
            total_invest_assets: self.total_invest_assets,
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

    pub fn close(self, reason: ClosePositionReason) -> ClosedPosition {
        ClosedPosition {
            pnl: None,
            asset_pnls: HashMap::new(),
            open_price: self.open_price,
            open_date: self.open_date,
            open_asset_prices: self.open_asset_prices,
            activate_date: None,
            activate_price: None,
            activate_asset_prices: HashMap::new(),
            close_date: DateTimeAsMicroseconds::now(),
            close_price: self.current_price,
            close_reason: reason,
            close_asset_prices: self.current_asset_prices.to_owned(),
            id: self.id,
            top_ups: Vec::with_capacity(0),
            total_invest_assets: self.total_invest_assets,
            order: self.order,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActivePosition {
    pub id: String,
    pub order: Order,
    pub open_price: f64,
    pub open_date: DateTimeAsMicroseconds,
    pub open_asset_prices: HashMap<String, f64>,
    pub activate_price: f64,
    pub activate_date: DateTimeAsMicroseconds,
    pub activate_asset_prices: HashMap<String, f64>,
    pub current_price: f64,
    pub current_asset_prices: HashMap<String, f64>,
    pub last_update_date: DateTimeAsMicroseconds,
    pub top_ups: Vec<ActiveTopUp>,
    pub current_pnl: f64,
    pub current_loss_percent: f64,
    pub prev_loss_percent: f64,
    pub top_up_locked: bool,
    pub total_invest_assets: HashMap<String, f64>,
}

impl ActivePosition {
    pub fn set_take_profit(&mut self, value: Option<TakeProfitConfig>) {
        self.order.take_profit = value;
    }

    pub fn set_stop_loss(&mut self, value: Option<StopLossConfig>) {
        self.order.stop_loss = value;
    }

    pub fn update(&mut self, bidask: &BidAsk) {
        self.try_update_instrument_price(bidask);
        self.try_update_asset_price(bidask);
        self.update_pnl();
    }

    pub fn try_cancel_top_ups(
        &mut self,
        price_change_percent: f64,
        delay: Duration,
    ) -> Vec<CanceledTopUp> {
        if self.top_ups.is_empty() {
            return Vec::with_capacity(0);
        }

        let mut canceled_top_ups = Vec::with_capacity(self.top_ups.len() / 3);

        let mut delay_start_date = DateTimeAsMicroseconds::now();
        delay_start_date.sub(delay);

        self.top_ups.retain(|top_up| {
            if top_up.date.is_later_than(delay_start_date) {
                return true;
            }

            let change_percent = price_change_percent / 100.0;

            if self.order.side == OrderSide::Buy
                && self.current_price < top_up.instrument_price * (1.0 + change_percent)
            {
                return true;
            }

            if self.order.side == OrderSide::Sell
                && self.current_price > top_up.instrument_price * (1.0 - change_percent)
            {
                return true;
            }

            for (asset_symbol, asset_amount) in top_up.assets.iter() {
                let invested_amount = self
                    .total_invest_assets
                    .get_mut(asset_symbol)
                    .expect("must exist: invalid top-up add");
                *invested_amount -= asset_amount;

                if *invested_amount <= 0.0 {
                    self.total_invest_assets.remove(asset_symbol);
                }
            }

            canceled_top_ups.push(top_up.to_owned().cancel(self.current_price));

            false
        });

        canceled_top_ups
    }

    fn try_update_instrument_price(&mut self, bidask: &BidAsk) {
        if self.order.instrument == bidask.instrument {
            self.current_price = bidask.get_close_price(&self.order.side)
        }
    }

    fn try_update_asset_price(&mut self, bidask: &BidAsk) {
        for asset in self.total_invest_assets.keys() {
            let id = BidAsk::generate_id(asset, &self.order.base_asset);

            if id == bidask.instrument {
                let price = bidask.get_asset_price(asset, &OrderSide::Sell);
                let current_asset_price = self.current_asset_prices.get_mut(asset);

                if let Some(current_asset_price) = current_asset_price {
                    *current_asset_price = price;
                } else {
                    self.current_asset_prices.insert(asset.to_owned(), price);
                }
            }
        }
    }

    pub fn close(self, reason: ClosePositionReason, pnl_accuracy: Option<u32>) -> ClosedPosition {
        let pnls_by_assets = self.calculate_pnls_by_assets(pnl_accuracy);
        let mut total_pnl = calculate_total_amount(&pnls_by_assets, &self.current_asset_prices);

        if let Some(pnl_accuracy) = pnl_accuracy {
            total_pnl = floor(total_pnl, pnl_accuracy);
        }

        ClosedPosition {
            total_invest_assets: self.total_invest_assets,
            pnl: Some(total_pnl),
            asset_pnls: pnls_by_assets,
            open_price: self.open_price,
            open_date: self.open_date,
            open_asset_prices: self.open_asset_prices,
            activate_date: Some(self.activate_date),
            activate_price: Some(self.activate_price),
            activate_asset_prices: self.activate_asset_prices,
            close_date: DateTimeAsMicroseconds::now(),
            close_price: self.current_price,
            close_reason: reason,
            close_asset_prices: self.current_asset_prices.to_owned(),
            order: self.order,
            id: self.id,
            top_ups: self.top_ups,
        }
    }

    pub fn determine_close_reason(&self) -> Option<ClosePositionReason> {
        if self.is_stop_out() {
            return Some(ClosePositionReason::StopOut);
        }

        if self.is_stop_loss() {
            return Some(ClosePositionReason::StopLoss);
        }

        if self.is_take_profit() {
            return Some(ClosePositionReason::TakeProfit);
        }

        None
    }

    pub fn try_close(self, pnl_accuracy: Option<u32>) -> Position {
        let Some(reason) = self.determine_close_reason() else {
            return Position::Active(self);
        };

        Position::Closed(self.close(reason, pnl_accuracy))
    }

    fn is_take_profit(&self) -> bool {
        if let Some(take_profit_config) = self.order.take_profit.as_ref() {
            take_profit_config.is_triggered(self.current_pnl, self.current_price, &self.order.side)
        } else {
            false
        }
    }

    fn is_stop_loss(&self) -> bool {
        if let Some(stop_loss_config) = self.order.stop_loss.as_ref() {
            stop_loss_config.is_triggered(self.current_pnl, self.current_price, &self.order.side)
        } else {
            false
        }
    }

    fn is_stop_out(&self) -> bool {
        if self.is_top_up() {
            return false;
        }

        self.current_loss_percent >= self.order.stop_out_percent
    }

    pub fn is_margin_call(&self) -> bool {
        if self.order.top_up_enabled {
            return false;
        }

        self.current_loss_percent >= self.order.margin_call_percent
            && self.prev_loss_percent < self.order.margin_call_percent
    }

    pub fn set_top_up_lock(&mut self, is_locked: bool) {
        self.top_up_locked = is_locked;
    }

    pub fn is_top_up(&self) -> bool {
        if self.top_up_locked {
            return false;
        }

        if !self.order.top_up_enabled {
            return false;
        }

        self.current_loss_percent >= self.order.top_up_percent
    }

    /// Calculates total asset amounts invested to position. Including order and all active top-ups
    pub fn _calculate_total_invest_assets(&self) -> HashMap<String, f64> {
        let mut amounts = HashMap::with_capacity(self.order.invest_assets.len() + 5);

        for (asset, amount) in self.order.invest_assets.iter() {
            let total_amount = amounts.get_mut(asset);
            if let Some(total_amount) = total_amount {
                *total_amount += amount;
            } else {
                amounts.insert(asset.to_owned(), amount.to_owned());
            }
        }

        for top_up in self.top_ups.iter() {
            for (asset, amount) in top_up.assets.iter() {
                let total_amount = amounts.get_mut(asset);
                if let Some(total_amount) = total_amount {
                    *total_amount += amount;
                } else {
                    amounts.insert(asset.to_owned(), amount.to_owned());
                }
            }
        }

        amounts
    }

    /// Calculates amount for next top-up in base asset
    pub fn calculate_required_top_up_amount(&self) -> f64 {
        if !self.is_top_up() {
            panic!("Position top-up is not possible")
        }

        let total_amount =
            calculate_total_amount(&self.total_invest_assets, &self.current_asset_prices);

        total_amount * self.order.top_up_percent / 100.0
    }

    /// Calculates total top-up amount in base asset by position
    pub fn _calculate_active_top_ups_amount(&self, asset_prices: &HashMap<String, f64>) -> f64 {
        let mut top_ups_amount = 0.0;

        for top_up in self.top_ups.iter() {
            top_ups_amount += calculate_total_amount(&top_up.assets, asset_prices);
        }

        top_ups_amount
    }

    /// Calculates total pnl in base asset by position
    fn calculate_pnl(&self, invest_amount: f64, initial_price: f64) -> f64 {
        let volume = self.order.calculate_volume(invest_amount);

        match self.order.side {
            OrderSide::Buy => (self.current_price / initial_price - 1.0) * volume,
            OrderSide::Sell => (self.current_price / initial_price - 1.0) * -volume,
        }
    }

    pub fn calculate_pnls_by_assets(&self, pnl_accuracy: Option<u32>) -> HashMap<String, f64> {
        let mut pnls_by_assets = HashMap::with_capacity(self.total_invest_assets.len());

        for (asset, amount) in self.total_invest_assets.iter() {
            let pnl = self.calculate_pnl(*amount, self.activate_price);
            let max_loss_amount = amount * -1.0; // limit for isolated trade

            let mut pnl = if pnl < max_loss_amount {
                max_loss_amount
            } else {
                pnl
            };

            if let Some(pnl_accuracy) = pnl_accuracy {
                pnl = floor(pnl, pnl_accuracy);
            }

            pnls_by_assets.insert(asset.to_owned(), pnl);
        }

        pnls_by_assets
    }

    pub fn add_top_up(&mut self, top_up: ActiveTopUp) {
        for (asset_symbol, asset_price) in top_up.asset_prices.iter() {
            self.current_asset_prices
                .insert(asset_symbol.to_owned(), asset_price.to_owned());
        }

        for (asset_symbol, asset_amount) in top_up.assets.iter() {
            let invested_asset_amount = self.total_invest_assets.get_mut(asset_symbol);

            if let Some(invested_asset_amount) = invested_asset_amount {
                *invested_asset_amount += asset_amount;
            } else {
                self.total_invest_assets
                    .insert(asset_symbol.to_owned(), *asset_amount);
            }
        }

        self.top_ups.push(top_up);
        self.update_pnl();
    }

    fn update_pnl(&mut self) {
        let total_asset_pnls = self.calculate_pnls_by_assets(None);
        self.current_pnl = calculate_total_amount(&total_asset_pnls, &self.current_asset_prices);
        self.prev_loss_percent = self.current_loss_percent;

        if self.current_pnl < 0.0 {
            let total_invest_amount =
                calculate_total_amount(&self.total_invest_assets, &self.current_asset_prices);
            self.current_loss_percent =
                calculate_percent(total_invest_amount, self.current_pnl.abs());
        } else {
            self.current_loss_percent = 0.0;
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClosedPosition {
    pub id: String,
    pub order: Order,
    pub open_price: f64,
    pub open_date: DateTimeAsMicroseconds,
    pub open_asset_prices: HashMap<String, f64>,
    pub activate_price: Option<f64>,
    pub activate_date: Option<DateTimeAsMicroseconds>,
    pub activate_asset_prices: HashMap<String, f64>,
    pub close_price: f64,
    pub close_date: DateTimeAsMicroseconds,
    pub close_reason: ClosePositionReason,
    pub close_asset_prices: HashMap<String, f64>,
    pub pnl: Option<f64>,
    pub asset_pnls: HashMap<String, f64>,
    pub top_ups: Vec<ActiveTopUp>,
    pub total_invest_assets: HashMap<String, f64>,
}

impl ClosedPosition {
    pub fn get_status(&self) -> PositionStatus {
        if self.activate_date.is_some() {
            PositionStatus::Filled
        } else {
            PositionStatus::Canceled
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ActivePosition, ClosePositionReason};
    use crate::{
        orders::{Order, OrderSide, TakeProfitConfig},
        positions::{BidAsk, Position},
    };
    use rust_extensions::date_time::DateTimeAsMicroseconds;
    use std::collections::HashMap;

    #[tokio::test]
    async fn close_active_position() {
        let order = Order {
            base_asset: "USDT".to_string(),
            id: "test".to_string(),
            instrument: "ATOMUSDT".to_string(),
            trader_id: "test".to_string(),
            wallet_id: "test".to_string(),
            created_date: DateTimeAsMicroseconds::now(),
            desire_price: None,
            funding_fee_period: None,
            invest_assets: HashMap::from([("BTC".to_string(), 100.0)]),
            leverage: 1.0,
            side: OrderSide::Buy,
            take_profit: None,
            stop_loss: None,
            stop_out_percent: 10.0,
            margin_call_percent: 10.0,
            top_up_enabled: false,
            top_up_percent: 10.0,
        };
        let prices = HashMap::from([("BTC".to_string(), 22300.0)]);
        let bidask = BidAsk {
            ask: 14.748,
            bid: 14.748,
            datetime: DateTimeAsMicroseconds::now(),
            instrument: "ATOMUSDT".to_string(),
        };
        let position = order.open(&bidask, &prices);
        let mut position = match position {
            Position::Active(position) => position,
            _ => {
                panic!("Invalid position")
            }
        };

        position.current_price = 14.75;
        let closed_position = position.close(ClosePositionReason::ClientCommand, None);

        let pnl = closed_position.pnl.unwrap();
        let asset_pnl = *closed_position.asset_pnls.get("BTC").unwrap();

        assert_ne!(pnl, asset_pnl);
        assert_eq!(302.41388662883173, pnl);
        assert_eq!(0.01356116083537362, asset_pnl);
    }

    #[tokio::test]
    async fn close_by_tp() {
        let instrument = "ATOMUSDT".to_string();
        let prices = HashMap::from([("USDT".to_string(), 1.0)]);
        let invest_assets = HashMap::from([("USDT".to_string(), 100342.0)]);
        let order = new_order(instrument, invest_assets, 1.0, OrderSide::Sell);
        let bidask = BidAsk {
            ask: 13.815,
            bid: 13.815,
            datetime: DateTimeAsMicroseconds::now(),
            instrument: "ATOMUSDT".to_string(),
        };
        let mut position = new_active_position(order, &bidask, &prices);
        let take_profit = TakeProfitConfig {
            unit: crate::orders::AutoClosePositionUnit::PriceRate,
            value: 13.817,
        };
        position.set_take_profit(Some(take_profit));
        position.current_price = 13.817;

        let position = position.try_close(None);
        let _position = match position {
            Position::Closed(position) => position,
            _ => panic!("must be closed"),
        };
    }

    fn new_order(
        instrument: String,
        invest_assets: HashMap<String, f64>,
        leverage: f64,
        side: OrderSide,
    ) -> Order {
        Order {
            base_asset: "USDT".to_string(),
            id: "test".to_string(),
            instrument,
            trader_id: "test".to_string(),
            wallet_id: "test".to_string(),
            created_date: DateTimeAsMicroseconds::now(),
            desire_price: None,
            funding_fee_period: None,
            invest_assets,
            leverage,
            side,
            take_profit: None,
            stop_loss: None,
            stop_out_percent: 90.0,
            margin_call_percent: 70.0,
            top_up_enabled: false,
            top_up_percent: 10.0,
        }
    }

    fn new_active_position(
        order: Order,
        bidask: &BidAsk,
        asset_prices: &HashMap<String, f64>,
    ) -> ActivePosition {
        let now = DateTimeAsMicroseconds::now();

        ActivePosition {
            id: Position::generate_id(),
            open_price: bidask.get_open_price(&order.side),
            open_date: now,
            open_asset_prices: asset_prices.to_owned(),
            activate_price: bidask.get_open_price(&order.side),
            activate_date: now,
            activate_asset_prices: asset_prices.to_owned(),
            current_price: bidask.get_close_price(&order.side),
            current_asset_prices: asset_prices.to_owned(),
            last_update_date: now,
            top_ups: Vec::new(),
            current_pnl: 0.0,
            current_loss_percent: 0.0,
            prev_loss_percent: 0.0,
            top_up_locked: false,
            total_invest_assets: order.invest_assets.clone(),
            order,
        }
    }
}
