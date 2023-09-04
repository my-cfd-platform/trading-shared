use crate::calculations::calculate_percent;
use crate::orders::OrderSide;
use crate::positions::BidAsk;
use ahash::AHashMap;

#[derive(Clone, Debug)]
pub struct Wallet {
    pub id: String,
    pub trader_id: String,
    pub total_balance: f64,
    pub margin_call_percent: f64,
    pub current_loss_percent: f64,
    prev_loss_percent: f64,
    estimate_asset: String,
    balances_by_instruments: AHashMap<String, WalletBalance>,
    estimated_amounts_by_balance_id: AHashMap<String, f64>,
    prices_by_assets: AHashMap<String, f64>,
    top_up_pnls_by_instruments: AHashMap<String, f64>,
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
            total_balance: 0.0,
            estimate_asset: estimate_asset.into(),
            balances_by_instruments: Default::default(),
            estimated_amounts_by_balance_id: Default::default(),
            prices_by_assets: Default::default(),
            margin_call_percent,
            current_loss_percent: 0.0,
            prev_loss_percent: 0.0,
            top_up_pnls_by_instruments: Default::default(),
        }
    }

    pub fn get_instruments(&self) -> Vec<&String> {
        self.balances_by_instruments.keys().collect()
    }

    pub fn set_instrument_pnl(&mut self, instrument: &str, instrument_pnl: f64) {
        let pnl = self.top_up_pnls_by_instruments.get_mut(instrument);

        if let Some(pnl) = pnl {
            *pnl = instrument_pnl;
        } else {
            self.top_up_pnls_by_instruments
                .insert(instrument.to_string(), instrument_pnl);
        }
    }

    pub fn deduct_instrument_pnl(&mut self, instrument: &str, instrument_pnl: f64) {
        let pnl = self.top_up_pnls_by_instruments.get_mut(instrument);

        if let Some(pnl) = pnl {
            *pnl -= instrument_pnl;
        }
    }

    pub fn add_instrument_pnl(&mut self, instrument: &str, instrument_pnl: f64) {
        let pnl = self.top_up_pnls_by_instruments.get_mut(instrument);

        if let Some(pnl) = pnl {
            *pnl += instrument_pnl;
        } else {
            self.top_up_pnls_by_instruments
                .insert(instrument.to_string(), instrument_pnl);
        }
    }

    pub fn calc_pnl(&self) -> f64 {
        self.top_up_pnls_by_instruments
            .iter()
            .map(|(_, pnl)| pnl)
            .sum()
    }

    pub fn update_loss(&mut self) {
        self.prev_loss_percent = self.current_loss_percent;
        let pnl: f64 = self.calc_pnl();

        if pnl < 0.0 {
            self.current_loss_percent = calculate_percent(self.total_balance, pnl.abs());
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

        let estimate_amount = if balance.asset_symbol == self.estimate_asset {
            self.prices_by_assets
                .insert(balance.asset_symbol.clone(), 1.0);
            balance.asset_amount
        } else {
            let price = bid_ask.get_asset_price(&balance.asset_symbol, &OrderSide::Sell);
            self.prices_by_assets
                .insert(balance.asset_symbol.clone(), price);
            balance.asset_amount * price
        };

        if !balance.is_locked {
            self.estimated_amounts_by_balance_id
                .insert(balance.id.clone(), estimate_amount);
            self.total_balance += estimate_amount;
        } else {
            self.estimated_amounts_by_balance_id
                .insert(balance.id.clone(), 0.0);
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
            let estimate_amount = self
                .estimated_amounts_by_balance_id
                .get_mut(&balance.id)
                .expect("invalid add");
            self.total_balance -= *estimate_amount;
            *estimate_amount = balance.asset_amount * price;
            self.total_balance *= *estimate_amount;
        }

        self.balances_by_instruments.insert(id, balance);

        Ok(())
    }

    pub fn update_price(&mut self, bid_ask: &BidAsk) {
        let balance = self.balances_by_instruments.get(&bid_ask.instrument);

        if let Some(balance) = balance {
            let estimate_balance_amount = self
                .estimated_amounts_by_balance_id
                .get_mut(&balance.id)
                .expect("invalid add or update");
            self.total_balance -= *estimate_balance_amount;
            let price = bid_ask.get_asset_price(&balance.asset_symbol, &OrderSide::Sell);
            *estimate_balance_amount = balance.asset_amount * price;
            self.total_balance += *estimate_balance_amount;
            let estimate_price = self
                .prices_by_assets
                .get_mut(&balance.asset_symbol)
                .expect("invalid add or update");
            *estimate_price = price;
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
