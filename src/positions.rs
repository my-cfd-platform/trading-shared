use crate::calculations::{calculate_percent, floor};
use crate::top_ups::{ActiveTopUp, CanceledTopUp};
use crate::{assets, calculations::calculate_total_amount, orders::{Order, OrderSide, StopLossConfig, TakeProfitConfig}};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use rust_extensions::date_time::DateTimeAsMicroseconds;
use std::time::Duration;
use compact_str::CompactString;
use rust_extensions::sorted_vec::SortedVec;
use uuid::Uuid;
use crate::asset_symbol::AssetSymbol;
use crate::assets::{AssetAmount, AssetPrice};
use crate::instrument_symbol::InstrumentSymbol;
use crate::position_id::PositionId;

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

#[derive(Clone, Debug)]
pub struct BidAsk {
    pub instrument: InstrumentSymbol,
    pub datetime: DateTimeAsMicroseconds,
    pub bid: f64,
    pub ask: f64,
}

impl BidAsk {
    pub fn new_synthetic(symbol: InstrumentSymbol, bid: f64, ask: f64) -> Self {
        Self {
            instrument: symbol,
            datetime: DateTimeAsMicroseconds::now(),
            bid,
            ask,
        }
    }

    pub fn get_instrument_symbol(base_asset: &AssetSymbol, quote_asset: &AssetSymbol) -> InstrumentSymbol {
        let mut compact_str = CompactString::with_capacity(base_asset.len() + quote_asset.len());
        compact_str.push_str(base_asset);
        compact_str.push_str(quote_asset);

        compact_str.into()
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

    pub fn get_asset_price(&self, asset: &AssetSymbol, side: &OrderSide) -> f64 {
        match side {
            OrderSide::Sell => {
                if self.instrument.0.starts_with(asset.0.as_str()) {
                    self.ask
                } else {
                    panic!("Invalid instrument {} for asset {}", self.instrument, asset)
                }
            }
            OrderSide::Buy => {
                if self.instrument.0.starts_with(asset.0.as_str()) {
                    self.bid
                } else {
                    panic!("Invalid instrument {} for asset {}", self.instrument, asset)
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Position {
    Active(ActivePosition),
    Closed(ClosedPosition),
    Pending(PendingPosition),
}

impl Position {
    pub fn generate_id() -> PositionId {
        Uuid::new_v4().into()
    }

    pub fn get_id(&self) -> &PositionId {
        match self {
            Position::Active(position) => &position.id,
            Position::Closed(position) => &position.id,
            Position::Pending(position) => &position.id,
        }
    }

    pub fn get_open_asset_prices(&self) -> &SortedVec<AssetSymbol, AssetPrice> {
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

    pub fn get_instruments(&self) -> Vec<InstrumentSymbol> {
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

    fn get_top_up_instruments(&self, top_ups: &Vec<ActiveTopUp>) -> Vec<InstrumentSymbol> {
        let mut instruments = Vec::with_capacity(10);

        for top_up in top_ups {
            for item in top_up.total_assets.iter() {
                let instrument = BidAsk::get_instrument_symbol(&item.symbol, &self.get_order().base_asset);

                if !instruments.contains(&instrument) {
                    instruments.push(instrument);
                }
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

#[derive(Debug, Clone)]
pub struct PendingPosition {
    pub id: PositionId,
    pub order: Order,
    pub open_price: f64,
    pub open_date: DateTimeAsMicroseconds,
    pub open_asset_prices: SortedVec<AssetSymbol, AssetPrice>,
    pub current_price: f64,
    pub current_asset_prices: SortedVec<AssetSymbol, AssetPrice>,
    pub last_update_date: DateTimeAsMicroseconds,
    pub total_invest_assets: SortedVec<AssetSymbol, AssetAmount>,
}

impl PendingPosition {
    pub fn update(&mut self, bidask: &BidAsk) {
        self.update_instrument_price(bidask);
        self.update_asset_prices(bidask);
        self.last_update_date = DateTimeAsMicroseconds::now();
    }

    pub fn is_price_reached(&self) -> bool {
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

        if is_stop_sell && self.current_price <= desired_price {
            return true;
        }

        let is_stop_buy = self.order.side == OrderSide::Buy && self.open_price <= desired_price;

        if is_stop_buy && self.current_price >= desired_price {
            return true;
        }

        false
    }

    fn update_instrument_price(&mut self, bidask: &BidAsk) {
        if self.order.instrument == bidask.instrument {
            self.current_price = bidask.get_open_price(&self.order.side)
        }
    }

    fn update_asset_prices(&mut self, bidask: &BidAsk) {
        for asset in self.order.invest_assets.iter() {
            let id = BidAsk::get_instrument_symbol(&asset.symbol, &self.order.base_asset);

            if id == bidask.instrument {
                let price = bidask.get_asset_price(&asset.symbol, &OrderSide::Sell);
                let current_asset_price = self.current_asset_prices.get_mut(&asset.symbol);

                if let Some(current_asset_price) = current_asset_price {
                    current_asset_price.price = price;
                } else {
                    self.current_asset_prices.insert_or_replace(AssetPrice::new(asset.symbol.clone(), price));
                }
            }
        }
    }

    pub fn can_activate(&self) -> bool {
        if self.total_invest_assets.is_empty() {
            return false;
        }

        if !self.is_price_reached() {
            return false;
        }

        true
    }

    pub fn try_activate(self) -> Position {
        if self.can_activate() {
            return Position::Active(self.activate().expect("checked in can_activate"));
        }

        Position::Pending(self)
    }

    pub fn activate(self) -> Result<ActivePosition, String> {
        if !self.is_price_reached() {
            return Err("desire_price isn't reached".to_string());
        }

        if self.total_invest_assets.is_empty() {
            return Err("total_invest_assets is empty".to_string());
        }

        let now = DateTimeAsMicroseconds::now();
        let mut order = self.order;
        order.invest_assets = self.total_invest_assets;

        Ok(ActivePosition {
            id: self.id,
            open_price: self.open_price,
            open_date: self.open_date,
            open_asset_prices: self.open_asset_prices,
            activate_price: self.current_price,
            activate_date: now,
            activate_asset_prices: self.current_asset_prices.to_owned(),
            current_price: self.current_price,
            current_asset_prices: self.current_asset_prices,
            last_update_date: now,
            top_ups: Vec::new(),
            current_pnl: 0.0,
            current_loss_percent: 0.0,
            prev_loss_percent: 0.0,
            top_up_locked: false,
            total_invest_assets: order.invest_assets.clone(),
            order,
            bonus_invest_assets: SortedVec::new(),
        })
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

    pub fn add_invest_assets(
        &mut self,
        amounts_by_assets: &SortedVec<AssetSymbol, AssetAmount>,
    ) -> Result<(), String> {
        for item in amounts_by_assets.iter() {
            if !self.open_asset_prices.contains(&item.symbol) {
                return Err(format!(
                    "Can't invest '{}': not found open price",
                    &item.symbol
                ));
            }

            let invested_asset_amount: Option<&mut AssetAmount> = self.total_invest_assets.get_mut(&item.symbol);

            if let Some(invested_asset_amount) = invested_asset_amount {
                invested_asset_amount.amount += item.amount;
            } else {
                self.total_invest_assets
                    .insert_or_replace(item.to_owned());
            }
        }

        Ok(())
    }

    pub fn close(self, reason: ClosePositionReason) -> ClosedPosition {
        ClosedPosition {
            pnl: None,
            asset_pnls: SortedVec::new(),
            open_price: self.open_price,
            open_date: self.open_date,
            open_asset_prices: self.open_asset_prices,
            activate_date: None,
            activate_price: None,
            activate_asset_prices: SortedVec::new(),
            close_date: DateTimeAsMicroseconds::now(),
            close_price: self.current_price,
            close_reason: reason,
            close_asset_prices: self.current_asset_prices.to_owned(),
            id: self.id,
            top_ups: Vec::with_capacity(0),
            total_invest_assets: self.total_invest_assets,
            order: self.order,
            invest_bonus_assets: SortedVec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActivePosition {
    pub id: PositionId,
    pub order: Order,
    pub open_price: f64,
    pub open_date: DateTimeAsMicroseconds,
    pub open_asset_prices: SortedVec<AssetSymbol, AssetPrice>,
    pub activate_price: f64,
    pub activate_date: DateTimeAsMicroseconds,
    pub activate_asset_prices: SortedVec<AssetSymbol, AssetPrice>,
    pub current_price: f64,
    pub current_asset_prices: SortedVec<AssetSymbol, AssetPrice>,
    pub last_update_date: DateTimeAsMicroseconds,
    pub top_ups: Vec<ActiveTopUp>,
    pub current_pnl: f64,
    pub current_loss_percent: f64,
    pub prev_loss_percent: f64,
    pub top_up_locked: bool,
    pub total_invest_assets: SortedVec<AssetSymbol, AssetAmount>,
    pub bonus_invest_assets: SortedVec<AssetSymbol, AssetAmount>,
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
        let delay_start_date = DateTimeAsMicroseconds::now();
        let delay_start_date = delay_start_date.sub(delay);

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

            for item in top_up.total_assets.iter() {
                let invested_amount = self
                    .total_invest_assets
                    .get_mut(&item.symbol)
                    .expect("must exist: invalid top-up add");
                invested_amount.amount -= item.amount;

                if invested_amount.amount <= 0.0 {
                    self.total_invest_assets.remove(&item.symbol);
                }
            }

            for item in top_up.bonus_assets.iter() {
                let invested_bonus = self
                    .bonus_invest_assets
                    .get_mut(&item.symbol)
                    .expect("must exist: invalid top-up add");
                invested_bonus.amount -= item.amount;

                if invested_bonus.amount <= 0.0 {
                    self.bonus_invest_assets.remove(&item.symbol);
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
        for asset in self.total_invest_assets.iter() {
            let id = BidAsk::get_instrument_symbol(&asset.symbol, &self.order.base_asset);

            if id == bidask.instrument {
                let price = bidask.get_asset_price(&asset.symbol, &OrderSide::Sell);
                let current_asset_price = self.current_asset_prices.get_mut(&asset.symbol);

                if let Some(current_asset_price) = current_asset_price {
                    current_asset_price.price = price;
                } else {
                    self.current_asset_prices.insert_or_replace(AssetPrice {price, symbol: asset.symbol.clone()});
                }
            }
        }
    }

    pub fn close(self, reason: ClosePositionReason, pnl_accuracy: Option<u32>) -> ClosedPosition {
        let pnls_by_assets = self.calc_pnls_by_assets(pnl_accuracy);
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
            invest_bonus_assets: self.bonus_invest_assets,
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

    /// Calculates amount for next top-up in base asset
    pub fn calculate_required_top_up_amount(&self) -> f64 {
        if !self.is_top_up() {
            panic!("Position top-up is not possible")
        }

        let total_amount =
            calculate_total_amount(&self.total_invest_assets, &self.current_asset_prices);

        total_amount * self.order.top_up_percent / 100.0
    }

    /// Calculates total pnl in base asset by position
    fn calculate_pnl(&self, invest_amount: f64, initial_price: f64) -> f64 {
        let volume = self.order.calculate_volume(invest_amount);

        match self.order.side {
            OrderSide::Buy => (self.current_price / initial_price - 1.0) * volume,
            OrderSide::Sell => (self.current_price / initial_price - 1.0) * -volume,
        }
    }

    pub fn add_top_up(&mut self, top_up: ActiveTopUp) {
        for item in top_up.asset_prices.iter() {
            self.current_asset_prices.insert_or_replace(item.clone());
        }

        for item in top_up.total_assets.iter() {
            let invested_asset_amount = self.total_invest_assets.get_mut(&item.symbol);

            if let Some(invested_asset_amount) = invested_asset_amount {
                invested_asset_amount.amount += item.amount;
            } else {
                self.total_invest_assets.insert_or_replace(item.clone());
            }
        }

        for item in top_up.bonus_assets.iter() {
            let bonus_asset_amount = self.bonus_invest_assets.get_mut(&item.symbol);

            if let Some(bonus_asset_amount) = bonus_asset_amount {
                bonus_asset_amount.amount += item.amount;
            } else {
                self.bonus_invest_assets.insert_or_replace(item.clone());
            }
        }

        self.top_ups.push(top_up);
        self.update_pnl();
    }

    fn update_pnl(&mut self) {
        let pnls_by_assets = self.calc_pnls_by_assets(None);
        self.current_pnl = calculate_total_amount(&pnls_by_assets, &self.current_asset_prices);
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

    /// Calculates total asset amounts invested to position. Including order and all active top-ups
    pub fn calc_total_invest_assets(&self) -> SortedVec<AssetSymbol, AssetAmount> {
        let mut amounts = SortedVec::new_with_capacity(self.order.invest_assets.len() + 5);

        for item in self.order.invest_assets.iter() {
            let total_amount: Option<&mut AssetAmount> = amounts.get_mut(&item.symbol);

            if let Some(total_amount) = total_amount {
                total_amount.amount += item.amount;
            } else {
                amounts.insert_or_replace(item.clone());
            }
        }

        for top_up in self.top_ups.iter() {
            for item in top_up.total_assets.iter() {
                let total_amount = amounts.get_mut(&item.symbol);

                if let Some(total_amount) = total_amount {
                    total_amount.amount += item.amount;
                } else {
                    amounts.insert_or_replace(item.clone());
                }
            }
        }

        amounts
    }

    /// Calculates pnl by all invested assets, includes order, and top-ups
    pub fn calc_pnls_by_assets(&self, pnl_accuracy: Option<u32>) -> SortedVec<AssetSymbol, AssetAmount> {
        let mut asset_pnls: SortedVec<AssetSymbol, AssetAmount> = SortedVec::new_with_capacity(self.order.invest_assets.len() + 5);

        for item in self.calc_order_pnls_by_assets().iter() {
            let asset_pnl: Option<&mut AssetAmount> = asset_pnls.get_mut(&item.symbol);

            if let Some(asset_pnl) = asset_pnl {
                asset_pnl.amount += item.amount;

                if let Some(pnl_accuracy) = pnl_accuracy {
                    asset_pnl.amount = floor(asset_pnl.amount, pnl_accuracy);
                };
            } else {
                let amount = if let Some(pnl_accuracy) = pnl_accuracy {
                    floor(item.amount, pnl_accuracy)
                } else {
                    item.amount
                };

                asset_pnls.insert_or_replace(assets::AssetAmount {symbol: item.symbol.clone(), amount});
            }
        }

        for item in self.calc_top_ups_pnls_by_assets().iter() {
            let asset_pnl: Option<&mut AssetAmount> = asset_pnls.get_mut(&item.symbol);

            if let Some(asset_pnl) = asset_pnl {
                asset_pnl.amount += item.amount;

                if let Some(pnl_accuracy) = pnl_accuracy {
                    asset_pnl.amount = floor(asset_pnl.amount, pnl_accuracy);
                };
            } else {
                let amount = if let Some(pnl_accuracy) = pnl_accuracy {
                    floor(item.amount, pnl_accuracy)
                } else {
                    item.amount
                };

                asset_pnls.insert_or_replace(assets::AssetAmount{symbol:item.symbol.clone(), amount});
            }
        }

        asset_pnls
    }

    /// Calculates pnl by invested assets initially in order
    pub fn calc_order_pnls_by_assets(&self) -> SortedVec<AssetSymbol, AssetAmount> {
        let mut pnls_by_assets = SortedVec::new_with_capacity(self.order.invest_assets.len());

        for item in self.order.invest_assets.iter() {
            let pnl = self.calculate_pnl(item.amount, self.activate_price);

            pnls_by_assets.insert_or_replace(assets::AssetAmount { amount:pnl, symbol: item.symbol.clone()});
        }

        pnls_by_assets
    }

    /// Calculates pnl by invested assets in top-ups
    pub fn calc_top_ups_pnls_by_assets(&self) -> SortedVec<AssetSymbol, AssetAmount> {
        let mut pnls_by_assets = SortedVec::new_with_capacity(10);

        for top_up in self.top_ups.iter() {
            for item in top_up.total_assets.iter() {
                let pnl = self.calculate_pnl(item.amount, top_up.instrument_price);
                let max_loss_amount = item.amount * -1.0; // limit for isolated trade
                let pnl = if pnl < max_loss_amount {
                    max_loss_amount
                } else {
                    pnl
                };

                let total_asset_pnl: Option<&mut AssetAmount> = pnls_by_assets.get_mut(&item.symbol);
                
                if let Some(total_asset_pnl) = total_asset_pnl {
                    total_asset_pnl.amount += pnl;
                } else {
                    pnls_by_assets.insert_or_replace(assets::AssetAmount {amount:pnl, symbol: item.symbol.clone()});
                }
            }
        }

        pnls_by_assets
    }
}

#[derive(Debug, Clone)]
pub struct ClosedPosition {
    pub id: PositionId,
    pub order: Order,
    pub open_price: f64,
    pub open_date: DateTimeAsMicroseconds,
    pub open_asset_prices: SortedVec<AssetSymbol, AssetPrice>,
    pub activate_price: Option<f64>,
    pub activate_date: Option<DateTimeAsMicroseconds>,
    pub activate_asset_prices: SortedVec<AssetSymbol, AssetPrice>,
    pub close_price: f64,
    pub close_date: DateTimeAsMicroseconds,
    pub close_reason: ClosePositionReason,
    pub close_asset_prices: SortedVec<AssetSymbol, AssetPrice>,
    pub pnl: Option<f64>,
    pub asset_pnls: SortedVec<AssetSymbol, AssetAmount>,
    pub top_ups: Vec<ActiveTopUp>,
    pub total_invest_assets: SortedVec<AssetSymbol, AssetAmount>,
    pub invest_bonus_assets: SortedVec<AssetSymbol, AssetAmount>,
}

impl ClosedPosition {
    pub fn get_status(&self) -> PositionStatus {
        if self.total_invest_assets.is_empty() {
            PositionStatus::Canceled
        } else {
            PositionStatus::Filled
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ActivePosition, ClosePositionReason};
    use crate::{assets, orders::{Order, OrderSide, TakeProfitConfig}, positions::{BidAsk, Position}};
    use rust_extensions::date_time::DateTimeAsMicroseconds;
    use rust_extensions::sorted_vec::SortedVec;
    use uuid::Uuid;
    use crate::asset_symbol::AssetSymbol;
    use crate::assets::{AssetAmount, AssetPrice};
    use crate::instrument_symbol::InstrumentSymbol;
    use crate::top_ups::ActiveTopUp;

    #[tokio::test]
    async fn close_active_position() {
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(assets::AssetAmount{ amount: 100.0, symbol: "BTC".into()});
        let order = Order {
            base_asset: "USDT".into(),
            id: "test".to_string(),
            instrument: "ATOMUSDT".into(),
            trader_id: "test".to_string(),
            wallet_id: Uuid::new_v4().into(),
            created_date: DateTimeAsMicroseconds::now(),
            desire_price: None,
            funding_fee_period: None,
            invest_assets,
            leverage: 1.0,
            side: OrderSide::Buy,
            take_profit: None,
            stop_loss: None,
            stop_out_percent: 10.0,
            margin_call_percent: 10.0,
            top_up_enabled: false,
            top_up_percent: 10.0,
        };
        let mut prices = SortedVec::new();
        prices.insert_or_replace(assets::AssetPrice{ price: 22300.0, symbol: "BTC".into()});
        let bidask = BidAsk {
            ask: 14.748,
            bid: 14.748,
            datetime: DateTimeAsMicroseconds::now(),
            instrument: "ATOMUSDT".into(),
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
        let asset_pnl = closed_position.asset_pnls.get(&AssetSymbol("BTC".into())).clone().unwrap();

        assert_ne!(pnl, asset_pnl.amount);
        assert_eq!(302.41388662883173, pnl);
        assert_eq!(0.01356116083537362, asset_pnl.amount);
    }

    #[tokio::test]
    async fn close_by_tp() {
        let instrument: InstrumentSymbol = "ATOMUSDT".into();
        let mut prices = SortedVec::new();
        prices.insert_or_replace(AssetPrice {price: 1.0, symbol: "USDT".into()});
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(assets::AssetAmount {amount: 100342.0, symbol: "USDT".into()});

        let order = new_order(instrument, invest_assets, 1.0, OrderSide::Sell);
        let bidask = BidAsk {
            ask: 13.815,
            bid: 13.815,
            datetime: DateTimeAsMicroseconds::now(),
            instrument: "ATOMUSDT".into(),
        };
        let mut position = new_active_position(order, &bidask, &prices);
        let take_profit = TakeProfitConfig {
            unit: crate::orders::AutoClosePositionUnit::PriceRateUnit,
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

    #[tokio::test]
    async fn calc_pnl_with_top_ups_2() {
        let instrument: InstrumentSymbol = "ATOMUSDT".into();
        let mut prices = SortedVec::new();
        prices.insert_or_replace(AssetPrice {price: 1.0, symbol: "USDT".into()});
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(assets::AssetAmount {amount: 100.0, symbol: "USDT".into()});

        let mut order = new_order(instrument.clone(), invest_assets, 10.0, OrderSide::Sell);
        order.top_up_enabled = true;
        let bidask = BidAsk {
            ask: 0.37,
            bid: 0.37,
            datetime: DateTimeAsMicroseconds::now(),
            instrument: instrument.clone(),
        };
        let mut position = new_active_position(order, &bidask, &prices);
        position.update(&BidAsk {
            ask: 0.37,
            bid: 0.37,
            datetime: DateTimeAsMicroseconds::now(),
            instrument,
        });

        assert_eq!(0.0, position.current_pnl);
    }

    #[tokio::test]
    async fn calc_pnl_with_top_ups() {
        let instrument: InstrumentSymbol = "ATOMUSDT".into();
        let mut prices = SortedVec::new();
        prices.insert_or_replace(AssetPrice {price: 1.0, symbol: "USDT".into()});
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(assets::AssetAmount {amount: 100.0, symbol: "USDT".into()});
        
        let mut order = new_order(instrument.clone(), invest_assets, 10.0, OrderSide::Sell);
        order.top_up_enabled = true;
        let bidask = BidAsk {
            ask: 0.33,
            bid: 0.33,
            datetime: DateTimeAsMicroseconds::now(),
            instrument: instrument.clone(),
        };

        let mut total_assets = SortedVec::new();
        total_assets.insert_or_replace(AssetAmount{ amount: 50.0, symbol: "USDT".into()});
        let mut position = new_active_position(order, &bidask, &prices);
        position.add_top_up(ActiveTopUp {
            id: "1".to_string(),
            date: DateTimeAsMicroseconds::now(),
            total_assets,
            instrument_price: 0.354,
            asset_prices: prices.clone(),
            bonus_assets: SortedVec::new(),
        });

        let mut total_assets = SortedVec::new();
        total_assets.insert_or_replace(AssetAmount{ amount: 75.0, symbol: "USDT".into()});
        position.add_top_up(ActiveTopUp {
            id: "2".to_string(),
            date: DateTimeAsMicroseconds::now(),
            total_assets,
            instrument_price: 0.355,
            asset_prices: prices.clone(),
            bonus_assets: SortedVec::new(),
        });
        
        let mut total_assets = SortedVec::new();
        total_assets.insert_or_replace(AssetAmount{ amount: 112.5, symbol: "USDT".into()});
        position.add_top_up(ActiveTopUp {
            id: "3".to_string(),
            date: DateTimeAsMicroseconds::now(),
            total_assets,
            instrument_price: 0.37,
            asset_prices: prices,
            bonus_assets: SortedVec::new(),
        });
        position.update(&BidAsk {
            ask: 0.37,
            bid: 0.37,
            datetime: DateTimeAsMicroseconds::now(),
            instrument,
        });

        println!("{}", position.current_pnl);

        assert_eq!(-175.50113211368867, position.current_pnl);
    }

    #[tokio::test]
    async fn stop_buy_not_reached() {
        let instrument: InstrumentSymbol = "ATOMUSDT".into();
        let mut prices = SortedVec::new();
        prices.insert_or_replace(AssetPrice {price: 1.0, symbol: "USDT".into()});
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(assets::AssetAmount {amount: 100342.0, symbol: "USDT".into()});
        let mut order = new_order(instrument.clone(), invest_assets, 1.0, OrderSide::Buy);
        order.desire_price = Some(26000.00);
        let bidask = BidAsk {
            ask: 25900.00,
            bid: 25900.00,
            datetime: DateTimeAsMicroseconds::now(),
            instrument,
        };
        let position = order.open(&bidask, &prices);
        let Position::Pending(pending_position) = position else {
            panic!("Must be pending position");
        };

        let is_price_reached = pending_position.is_price_reached();

        assert!(!is_price_reached);
    }

    #[tokio::test]
    async fn stop_buy_reached() {
        let instrument: InstrumentSymbol = "ATOMUSDT".into();
        let mut prices = SortedVec::new();
        prices.insert_or_replace(AssetPrice {price: 1.0, symbol: "USDT".into()});
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(assets::AssetAmount {amount: 100342.0, symbol: "USDT".into()});
        
        let mut order = new_order(instrument.clone(), invest_assets, 1.0, OrderSide::Buy);
        order.desire_price = Some(26000.00);
        let bidask = BidAsk {
            ask: 25900.00,
            bid: 25900.00,
            datetime: DateTimeAsMicroseconds::now(),
            instrument,
        };
        let position = order.open(&bidask, &prices);
        let Position::Pending(mut pending_position) = position else {
            panic!("Must be pending position");
        };
        pending_position.current_price = 26100.00;

        let is_price_reached = pending_position.is_price_reached();

        assert!(is_price_reached);
    }

    #[tokio::test]
    async fn limit_buy_reached() {
        let instrument: InstrumentSymbol = "ATOMUSDT".into();
        let mut prices = SortedVec::new();
        prices.insert_or_replace(AssetPrice {price: 1.0, symbol: "USDT".into()});
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(assets::AssetAmount {amount: 100342.0, symbol: "USDT".into()});
        
        let mut order = new_order(instrument.clone(), invest_assets, 1.0, OrderSide::Buy);
        order.desire_price = Some(25000.00);
        let bidask = BidAsk {
            ask: 25900.00,
            bid: 25900.00,
            datetime: DateTimeAsMicroseconds::now(),
            instrument,
        };
        let position = order.open(&bidask, &prices);
        let Position::Pending(mut pending_position) = position else {
            panic!("Must be pending position");
        };
        pending_position.current_price = 24100.00;

        let is_price_reached = pending_position.is_price_reached();

        assert!(is_price_reached);
    }

    #[tokio::test]
    async fn limit_buy_not_reached() {
        let instrument: InstrumentSymbol = "ATOMUSDT".into();
        let mut prices = SortedVec::new();
        prices.insert_or_replace(assets::AssetPrice {price: 1.0, symbol: "USDT".into()});
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(assets::AssetAmount {amount: 100342.0, symbol: "USDT".into()});
        
        let mut order = new_order(instrument.clone(), invest_assets, 1.0, OrderSide::Buy);
        order.desire_price = Some(25000.00);
        let bidask = BidAsk {
            ask: 25900.00,
            bid: 25900.00,
            datetime: DateTimeAsMicroseconds::now(),
            instrument,
        };
        let position = order.open(&bidask, &prices);
        let Position::Pending(mut pending_position) = position else {
            panic!("Must be pending position");
        };
        pending_position.current_price = 26100.00;

        let is_price_reached = pending_position.is_price_reached();

        assert!(!is_price_reached);
    }

    #[tokio::test]
    async fn limit_sell_not_reached() {
        let instrument: InstrumentSymbol = "ATOMUSDT".into();
        let mut prices = SortedVec::new();
        prices.insert_or_replace(assets::AssetPrice {price: 1.0, symbol: "USDT".into()});
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(assets::AssetAmount {amount: 100342.0, symbol: "USDT".into()});
        
        let mut order = new_order(instrument.clone(), invest_assets, 1.0, OrderSide::Sell);
        order.desire_price = Some(26000.00);
        let bidask = BidAsk {
            ask: 25900.00,
            bid: 25900.00,
            datetime: DateTimeAsMicroseconds::now(),
            instrument,
        };
        let position = order.open(&bidask, &prices);
        let Position::Pending(pending_position) = position else {
            panic!("Must be pending position");
        };

        let is_price_reached = pending_position.is_price_reached();

        assert!(!is_price_reached);
    }

    #[tokio::test]
    async fn limit_sell_reached() {
        let instrument: InstrumentSymbol = "ATOMUSDT".into();
        let mut prices = SortedVec::new();
        prices.insert_or_replace(assets::AssetPrice {price: 1.0, symbol: "USDT".into()});
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(assets::AssetAmount {amount: 100342.0, symbol: "USDT".into()});
        
        let mut order = new_order(instrument.clone(), invest_assets, 1.0, OrderSide::Sell);
        order.desire_price = Some(26000.00);
        let bidask = BidAsk {
            ask: 25900.00,
            bid: 25900.00,
            datetime: DateTimeAsMicroseconds::now(),
            instrument,
        };
        let position = order.open(&bidask, &prices);
        let Position::Pending(mut pending_position) = position else {
            panic!("Must be pending position");
        };
        pending_position.current_price = 26100.00;

        let is_price_reached = pending_position.is_price_reached();

        assert!(is_price_reached);
    }

    #[tokio::test]
    async fn stop_sell_reached() {
        let instrument: InstrumentSymbol = "ATOMUSDT".into();
        let mut prices = SortedVec::new();
        prices.insert_or_replace(assets::AssetPrice {price: 1.0, symbol: "USDT".into()});
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(assets::AssetAmount {amount: 100342.0, symbol: "USDT".into()});
        
        let mut order = new_order(instrument.clone(), invest_assets, 1.0, OrderSide::Sell);
        order.desire_price = Some(25000.00);
        let bidask = BidAsk {
            ask: 25900.00,
            bid: 25900.00,
            datetime: DateTimeAsMicroseconds::now(),
            instrument,
        };
        let position = order.open(&bidask, &prices);
        let Position::Pending(mut pending_position) = position else {
            panic!("Must be pending position");
        };
        pending_position.current_price = 24900.00;

        let is_price_reached = pending_position.is_price_reached();

        assert!(is_price_reached);
    }

    #[tokio::test]
    async fn stop_sell_not_reached() {
        let instrument: InstrumentSymbol = "ATOMUSDT".into();
        let mut prices = SortedVec::new();
        prices.insert_or_replace(assets::AssetPrice {price: 1.0, symbol: "USDT".into()});
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(assets::AssetAmount {amount: 100342.0, symbol: "USDT".into()});
        
        let mut order = new_order(instrument.clone(), invest_assets, 1.0, OrderSide::Sell);
        order.desire_price = Some(25000.00);
        let bidask = BidAsk {
            ask: 25900.00,
            bid: 25900.00,
            datetime: DateTimeAsMicroseconds::now(),
            instrument,
        };
        let position = order.open(&bidask, &prices);
        let Position::Pending(mut pending_position) = position else {
            panic!("Must be pending position");
        };
        pending_position.current_price = 25900.00;

        let is_price_reached = pending_position.is_price_reached();

        assert!(!is_price_reached);
    }

    fn new_order(
        instrument: InstrumentSymbol,
        invest_assets: SortedVec<AssetSymbol, assets::AssetAmount>,
        leverage: f64,
        side: OrderSide,
    ) -> Order {
        Order {
            base_asset: "USDT".into(),
            id: "test".to_string(),
            instrument,
            trader_id: "test".to_string(),
            wallet_id: Uuid::new_v4().into(),
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
        asset_prices: &SortedVec<AssetSymbol, AssetPrice>,
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
            bonus_invest_assets: SortedVec::new(),
        }
    }
}
