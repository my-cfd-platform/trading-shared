use crate::calculations::calculate_percent;
use crate::orders::OrderSide;
use crate::positions::BidAsk;
use ahash::AHashMap;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Wallet {
    pub id: String,
    pub trader_id: String,
    pub total_unlocked_balance: f64,
    pub margin_call_percent: f64,
    pub current_loss_percent: f64,
    prev_loss_percent: f64,
    estimate_asset: String,
    balances_by_instruments: AHashMap<String, WalletBalance>,
    prices_by_assets: AHashMap<String, f64>,
    top_up_pnls_by_instruments: AHashMap<String, f64>,
    top_up_reserved_balance_by_instruments: AHashMap<String, f64>,
    pub total_top_up_reserved_balance: f64,
}

impl Wallet {
    pub fn new(
        id: impl Into<String>,
        trader_id: impl Into<String>,
        estimate_asset: impl Into<String>,
        margin_call_percent: f64,
    ) -> Self {
        Self {
            id: id.into(),
            trader_id: trader_id.into(),
            total_unlocked_balance: 0.0,
            estimate_asset: estimate_asset.into(),
            balances_by_instruments: Default::default(),
            prices_by_assets: Default::default(),
            margin_call_percent,
            current_loss_percent: 0.0,
            prev_loss_percent: 0.0,
            top_up_pnls_by_instruments: Default::default(),
            top_up_reserved_balance_by_instruments: Default::default(),
            total_top_up_reserved_balance: 0.0,
        }
    }

    pub fn set_top_up_reserved(
        &mut self,
        instrument: &str,
        instrument_reserved: &HashMap<String, f64>,
    ) {
        let mut new_reserved = 0.0;

        for (asset_symbol, asset_amount) in instrument_reserved.iter() {
            let price = self.prices_by_assets.get(asset_symbol);

            if let Some(price) = price {
                new_reserved += price * asset_amount;
            }
        }

        let old_reserved = self
            .top_up_reserved_balance_by_instruments
            .get_mut(instrument);

        if let Some(old_reserved) = old_reserved {
            self.total_top_up_reserved_balance -= *old_reserved;
            *old_reserved = new_reserved;
        } else {
            self.total_top_up_reserved_balance += new_reserved;
            self.top_up_reserved_balance_by_instruments
                .insert(instrument.to_string(), new_reserved);
        }
    }

    pub fn get_instruments(&self) -> Vec<&String> {
        self.balances_by_instruments.keys().collect()
    }

    pub fn set_top_up_pnl(&mut self, instrument: &str, instrument_pnl: f64) {
        self.top_up_pnls_by_instruments
            .insert(instrument.to_string(), instrument_pnl);
    }

    pub fn deduct_top_up_pnl(&mut self, instrument: &str, instrument_pnl: f64) {
        let pnl = self.top_up_pnls_by_instruments.get_mut(instrument);

        if let Some(pnl) = pnl {
            *pnl -= instrument_pnl;
        }
    }

    pub fn add_top_up_pnl(&mut self, instrument: &str, instrument_pnl: f64) {
        let pnl = self.top_up_pnls_by_instruments.get_mut(instrument);

        if let Some(pnl) = pnl {
            *pnl += instrument_pnl;
        } else {
            self.top_up_pnls_by_instruments
                .insert(instrument.to_string(), instrument_pnl);
        }
    }

    pub fn calc_total_pnl(&self) -> f64 {
        self.top_up_pnls_by_instruments
            .iter()
            .map(|(_, pnl)| pnl)
            .sum()
    }

    pub fn update_loss(&mut self) {
        self.prev_loss_percent = self.current_loss_percent;
        let pnl: f64 = self.calc_total_pnl();

        if pnl < 0.0 {
            self.current_loss_percent = calculate_percent(
                self.total_unlocked_balance + self.total_top_up_reserved_balance,
                pnl.abs(),
            );
        } else {
            self.current_loss_percent = 0.0;
        }
    }

    pub fn is_margin_call(&self) -> bool {
        self.current_loss_percent >= self.margin_call_percent
            && self.prev_loss_percent < self.margin_call_percent
    }

    pub fn add_balance(&mut self, balance: WalletBalance, bid_ask: &BidAsk) -> Result<(), String> {
        let instrument_id = BidAsk::generate_id(&balance.asset_symbol, &self.estimate_asset);

        if bid_ask.instrument != instrument_id {
            return Err(format!("BidAsk instrument must be {}", instrument_id));
        }

        let price = bid_ask.get_asset_price(&balance.asset_symbol, &OrderSide::Sell);
        self.prices_by_assets
            .insert(balance.asset_symbol.clone(), price);
        let estimate_amount = balance.asset_amount * price;

        if !balance.is_locked {
            self.total_unlocked_balance += estimate_amount;
        }

        self.balances_by_instruments.insert(instrument_id, balance);

        Ok(())
    }

    pub fn update_balance(&mut self, balance: WalletBalance) -> Result<(), String> {
        let id = BidAsk::generate_id(&balance.asset_symbol, &self.estimate_asset);
        let inner_balance = self.balances_by_instruments.remove(&id);

        let Some(inner_balance) = inner_balance else {
            return Err("Balance not found".to_string());
        };

        if !balance.is_locked {
            let price = self
                .prices_by_assets
                .get(&inner_balance.asset_symbol)
                .expect("invalid add");
            self.total_unlocked_balance -= inner_balance.asset_amount * price;
            self.total_unlocked_balance += balance.asset_amount * price;
        }

        self.balances_by_instruments.insert(id, balance);

        Ok(())
    }

    pub fn set_balance_lock(&mut self, balance_id: &str, is_locked: bool) -> Result<(), String> {
        let inner_balance = self
            .balances_by_instruments
            .values_mut()
            .find(|b| b.id == balance_id);

        let Some(balance) = inner_balance else {
            return Err("Balance not found".to_string());
        };

        if balance.is_locked == is_locked {
            return Ok(()); // no changes no need to do anything
        }

        let price = self
            .prices_by_assets
            .get(&balance.asset_symbol)
            .expect("invalid add");

        if !balance.is_locked && is_locked {
            self.total_unlocked_balance -= balance.asset_amount * price;
        } else if balance.is_locked && !is_locked {
            self.total_unlocked_balance += balance.asset_amount * price;
        }

        balance.is_locked = is_locked;

        Ok(())
    }

    pub fn update_price(&mut self, bid_ask: &BidAsk) {
        let balance = self.balances_by_instruments.get(&bid_ask.instrument);

        if let Some(balance) = balance {
            let new_price = bid_ask.get_asset_price(&balance.asset_symbol, &OrderSide::Sell);
            let old_price = self
                .prices_by_assets
                .get_mut(&balance.asset_symbol)
                .expect("invalid add or update");

            if balance.is_locked {
                self.total_unlocked_balance -= balance.asset_amount * *old_price;
                self.total_unlocked_balance += balance.asset_amount * new_price;
            }

            *old_price = new_price;
        }
    }
}

#[derive(Clone, Debug)]
pub struct WalletBalance {
    pub id: String,
    pub asset_symbol: String,
    pub asset_amount: f64,
    pub is_locked: bool,
}
