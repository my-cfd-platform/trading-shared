use crate::positions::{BidAsk, Position};
use std::{
    collections::{HashMap, HashSet},
    mem,
    sync::Arc,
};

pub struct BidAsksCache {
    bidasks_by_instruments: HashMap<String, BidAsk>,
}

impl BidAsksCache {
    pub fn new(bidasks_by_instruments: HashMap<String, BidAsk>) -> Self {
        Self {
            bidasks_by_instruments,
        }
    }

    pub fn update(&mut self, bidask: BidAsk) {
        let current_bidask = self.bidasks_by_instruments.get_mut(&bidask.instrument);

        if let Some(current_bidask) = current_bidask {
            _ = mem::replace(current_bidask, bidask);
        } else {
            self.bidasks_by_instruments
                .insert(bidask.instrument.clone(), bidask);
        }
    }

    pub fn get(&self, instrument: &str) -> Option<&BidAsk> {
        let bidask = self.bidasks_by_instruments.get(instrument);

        return bidask;
    }

    pub fn find(&self, base_asset: &str, assets: &[&String]) -> HashMap<String, BidAsk> {
        let mut bidasks = HashMap::with_capacity(assets.len());

        for asset in assets.iter() {
            let instrument = BidAsk::generate_id(asset, base_asset);
            let bidask = self.bidasks_by_instruments.get(&instrument);

            if let Some(bidask) = bidask {
                bidasks.insert(instrument, bidask.clone());
            }
        }

        return bidasks;
    }

    pub fn find_prices(&self, to_asset: &str, from_assets: &[&String]) -> HashMap<String, f64> {
        let mut prices = HashMap::with_capacity(from_assets.len());

        for asset in from_assets.into_iter() {
            if *asset == to_asset {
                prices.insert((*asset).to_owned(), 1.0);
            }

            let instrument = BidAsk::generate_id(asset, to_asset);
            let bidask = self.bidasks_by_instruments.get(&instrument);

            if let Some(bidask) = bidask {
                let price = bidask.get_asset_price(asset, &crate::orders::OrderSide::Sell);
                prices.insert((*asset).to_owned(), price);
            }
        }

        return prices;
    }
}

pub struct PositionsByIds {
    positions_by_ids: HashMap<String, Arc<Position>>,
}

impl PositionsByIds {
    pub fn new(position: &Arc<Position>) -> Self {
        Self {
            positions_by_ids: HashMap::from([(position.get_id().to_owned(), Arc::clone(position))]),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.positions_by_ids.is_empty()
    }

    pub fn get_all(&self) -> &HashMap<String, Arc<Position>> {
        &self.positions_by_ids
    }

    pub fn get_all_positions(&self) -> Vec<&Arc<Position>> {
        self.positions_by_ids.values().collect()
    }

    pub fn add_or_replace(&mut self, position: &Arc<Position>) {
        let position_id = position.get_id();
        let mut position = Arc::clone(position);

        if let Some(existing) = self.positions_by_ids.get_mut(position_id) {
            mem::swap(existing, &mut position);
        } else {
            self.positions_by_ids
                .insert(position_id.to_owned(), position);
        }
    }

    pub fn remove(&mut self, id: &str) -> Option<Arc<Position>> {
        let position = self.positions_by_ids.remove(id);

        if let Some(position) = position {
            Some(position)
        } else {
            None
        }
    }
}

pub struct PositionsCache {
    positions_by_wallets: HashMap<String, PositionsByIds>,
    positions_by_order_instruments: HashMap<String, PositionsByIds>,
    positions_by_invest_intruments: HashMap<String, PositionsByIds>,
}

impl PositionsCache {
    pub fn new() -> PositionsCache {
        PositionsCache {
            positions_by_wallets: HashMap::new(),
            positions_by_order_instruments: HashMap::new(),
            positions_by_invest_intruments: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        if !self.positions_by_wallets.is_empty() {
            for item in self.positions_by_wallets.values() {
                if !item.is_empty() {
                    return false;
                }
            }
        }

        if !self.positions_by_order_instruments.is_empty() {
            for item in self.positions_by_order_instruments.values() {
                if !item.is_empty() {
                    return false;
                }
            }
        }

        if !self.positions_by_invest_intruments.is_empty() {
            for item in self.positions_by_invest_intruments.values() {
                if !item.is_empty() {
                    return false;
                }
            }
        }

        return true;
    }

    pub fn add(&mut self, position: Position) {
        let position = Arc::new(position);

        // add by wallet id
        let wallet_positions = self
            .positions_by_wallets
            .get_mut(&position.get_order().wallet_id);

        match wallet_positions {
            Some(positions) => {
                positions.add_or_replace(&position);
            }
            None => {
                let wallet_id = position.get_order().wallet_id.clone();
                self.positions_by_wallets
                    .insert(wallet_id, PositionsByIds::new(&position));
            }
        }

        // add by instrument
        let instrument_positions = self
            .positions_by_order_instruments
            .get_mut(&position.get_order().instrument);

        match instrument_positions {
            Some(positions) => {
                positions.add_or_replace(&position);
            }
            None => {
                let instrument = position.get_order().instrument.clone();
                self.positions_by_order_instruments
                    .insert(instrument, PositionsByIds::new(&position));
            }
        }

        // add by invest instruments
        for instrument in position.get_order().get_invest_instruments() {
            let asset_positions = self.positions_by_invest_intruments.get_mut(&instrument);

            match asset_positions {
                Some(positions) => {
                    positions.add_or_replace(&position);
                }
                None => {
                    self.positions_by_invest_intruments
                        .insert(instrument, PositionsByIds::new(&position));
                }
            }
        }
    }

    pub fn get_by_wallet_id(&self, wallet_id: &str) -> Vec<Arc<Position>> {
        let wallet_positions = self.positions_by_wallets.get(wallet_id);

        if let Some(wallet_positions) = wallet_positions {
            return wallet_positions
                .get_all_positions()
                .into_iter()
                .map(|p| Arc::clone(p))
                .collect();
        }

        Vec::new()
    }

    pub fn get_by_instrument(&self, instrument: &str) -> Vec<Arc<Position>> {
        let mut found_position_ids = HashSet::new();
        let mut all_positions: Vec<Arc<Position>> = Vec::new();
        let positions = self.positions_by_order_instruments.get(instrument);

        if let Some(positions) = positions {
            let positions = positions.get_all();

            for (id, position) in positions {
                all_positions.push(Arc::clone(position));
                found_position_ids.insert(id);
            }
        }

        let positions = self.positions_by_invest_intruments.get(instrument);

        if let Some(positions) = positions {
            let positions = positions.get_all();

            for (id, position) in positions {
                if !found_position_ids.contains(id) {
                    all_positions.push(Arc::clone(position));
                    found_position_ids.insert(id);
                }
            }
        }

        all_positions
    }

    pub fn remove(&mut self, position_id: &str, wallet_id: &str) -> Option<Position> {
        let wallet_positions = self.positions_by_wallets.get_mut(wallet_id);

        let position = match wallet_positions {
            Some(positions) => {
                let position = positions.remove(position_id);

                if let Some(position) = position {
                    let order = position.get_order();
                    let instrument_positions = self
                        .positions_by_order_instruments
                        .get_mut(&order.instrument);

                    if let Some(instrument_positions) = instrument_positions {
                        instrument_positions.remove(position_id);
                    }

                    for instrument in order.get_invest_instruments() {
                        let asset_positions =
                            self.positions_by_invest_intruments.get_mut(&instrument);

                        if let Some(asset_positions) = asset_positions {
                            asset_positions.remove(position_id);
                        }
                    }

                    Some(position.as_ref().to_owned())
                } else {
                    None
                }
            }
            None => None,
        };

        position
    }
}

#[cfg(test)]
mod tests {
    use super::PositionsCache;
    use crate::{caches::PositionsByIds, orders::Order, positions::Position};
    use std::{collections::HashMap, sync::Arc};

    #[test]
    fn positions_cache_is_empty() {
        let cache = PositionsCache::new();

        assert!(cache.is_empty());
    }

    #[test]
    fn positions_cache_not_empty() {
        let position = new_position();
        let cache = PositionsCache {
            positions_by_wallets: HashMap::from([(
                "s".to_string(),
                PositionsByIds::new(&Arc::new(position)),
            )]),
            positions_by_order_instruments: HashMap::new(),
            positions_by_invest_intruments: HashMap::new(),
        };

        assert!(!cache.is_empty());
    }

    #[test]
    fn positions_cache_add() {
        let position = new_position();
        let mut cache = PositionsCache::new();

        cache.add(position.clone());

        assert!(!cache.is_empty());
    }

    #[test]
    fn positions_cache_remove() {
        let position = new_position();
        let order = position.get_order();
        let mut cache = PositionsCache::new();

        cache.add(position.clone());
        cache.remove(position.get_id(), &order.wallet_id);

        assert!(cache.is_empty());
    }

    #[test]
    fn positions_cache_get_by_wallet() {
        let position = new_position();
        let mut cache = PositionsCache::new();

        cache.add(position.clone());
        let order = position.get_order();
        let positions = cache.get_by_wallet_id(&order.wallet_id);

        assert!(!positions.is_empty());
    }

    #[test]
    fn positions_cache_get_by_invest_instrument() {
        let position = new_position();
        let mut cache = PositionsCache::new();

        cache.add(position.clone());
        let order = position.get_order();

        for instrument in order.get_invest_instruments() {
            let positions = cache.get_by_invest_instrument(&instrument);

            assert!(!positions.is_empty());
        }
    }

    #[test]
    fn positions_cache_get_by_instrument() {
        let position = new_position();
        let mut cache = PositionsCache::new();

        cache.add(position.clone());
        let order = position.get_order();
        let positions = cache.get_by_instrument(&order.instrument);

        assert!(!positions.is_empty());
    }

    fn new_position() -> Position {
        let invest_asset = ("BTC".to_string(), 100.0);
        let order = Order {
            base_asset: "USDT".to_string(),
            id: "test".to_string(),
            instrument: "ATOMUSDT".to_string(),
            trader_id: "test".to_string(),
            wallet_id: "test".to_string(),
            created_date: rust_extensions::date_time::DateTimeAsMicroseconds::now(),
            desire_price: None,
            funding_fee_period: None,
            invest_assets: HashMap::from([invest_asset.clone()]),
            leverage: 1.0,
            side: crate::orders::OrderSide::Buy,
            take_profit: None,
            stop_loss: None,
            stop_out_percent: 10.0,
            margin_call_percent: 10.0,
            top_up_enabled: false,
            top_up_percent: 10.0,
        };
        let prices = HashMap::from([("BTC".to_string(), 22300.0)]);
        let position = order.open(14.748, &prices);

        position
    }
}
