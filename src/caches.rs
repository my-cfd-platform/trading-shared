use std::mem;
use crate::positions::{BidAsk, Position};
use ahash::{AHashMap, AHashSet};

pub struct BidAsksCache {
    bidasks_by_instruments: AHashMap<String, BidAsk>,
}

impl BidAsksCache {
    pub fn new(bidasks: Vec<BidAsk>) -> Self {
        let mut map = AHashMap::with_capacity(bidasks.len());

        for bidask in bidasks.into_iter() {
            map.insert(bidask.instrument.clone(), bidask);
        }

        Self {
            bidasks_by_instruments: map,
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
        self.bidasks_by_instruments.get(instrument)
    }

    pub fn find(&self, base_asset: &str, assets: &[&String]) -> AHashMap<String, BidAsk> {
        let mut bidasks = AHashMap::with_capacity(assets.len());

        for asset in assets.iter() {
            let instrument = BidAsk::generate_id(asset, base_asset);
            let bidask = self.bidasks_by_instruments.get(&instrument);

            if let Some(bidask) = bidask {
                bidasks.insert(instrument, bidask.clone());
            }
        }

        bidasks
    }

    pub fn find_prices(&self, to_asset: &str, from_assets: &[&String]) -> AHashMap<String, f64> {
        let mut prices = AHashMap::with_capacity(from_assets.len());

        for asset in from_assets.iter() {
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

        prices
    }
}

pub struct PositionsCache {
    positions_by_ids: AHashMap<String, Position>,
    ids_by_wallets: AHashMap<String, AHashSet<String>>,
}

impl PositionsCache {
    pub fn with_capacity(capacity: usize) -> PositionsCache {
        PositionsCache {
            ids_by_wallets: AHashMap::with_capacity(capacity),
            positions_by_ids: AHashMap::with_capacity(capacity),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.positions_by_ids.is_empty()
    }

    pub fn add(&mut self, position: Position) {
        let id = position.get_id().to_owned();
        let wallet_id = position.get_order().wallet_id.clone();

        self.positions_by_ids.insert(id.clone(), position);

        if let Some(ids) = self.ids_by_wallets.get_mut(&wallet_id) {
            ids.insert(id);
        } else {
            self.ids_by_wallets
                .insert(wallet_id, AHashSet::from([id]));
        }
    }

    pub fn get_by_wallet_id(&self, wallet_id: &str) -> Vec<&Position> {
        let ids = self.ids_by_wallets.get(wallet_id);

        if let Some(ids) = ids {
            let mut positions = Vec::with_capacity(ids.len());

            for id in ids {
                positions.push(self.positions_by_ids.get(id).expect("Error in add method"));
            }

            return positions;
        }

        Vec::with_capacity(0)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Position> {
        self.positions_by_ids.get_mut(id)
    }

    pub fn remove(&mut self, position_id: &str) -> Option<Position> {
        let position = self.positions_by_ids.remove(position_id);

        if let Some(position) = position.as_ref() {
            if let Some(ids) = self.ids_by_wallets.get_mut(&position.get_order().wallet_id) {
                ids.remove(position_id);
            }
        }

        position
    }
}

#[cfg(test)]
mod tests {
    use rust_extensions::date_time::DateTimeAsMicroseconds;

    use super::PositionsCache;
    use crate::{
        orders::Order,
        positions::{BidAsk, Position},
    };
    use std::collections::HashMap;

    #[test]
    fn positions_cache_is_empty() {
        let cache = PositionsCache::with_capacity(10);

        assert!(cache.is_empty());
    }

    #[test]
    fn positions_cache_add() {
        let position = new_position();
        let mut cache = PositionsCache::with_capacity(10);

        cache.add(position);

        assert!(!cache.is_empty());
    }

    #[test]
    fn positions_cache_remove() {
        let position = new_position();
        let order = position.get_order();
        let mut cache = PositionsCache::with_capacity(10);

        cache.add(position.clone());
        assert!(!cache.is_empty());

        cache.remove(position.get_id());
        let positions = cache.get_by_wallet_id(&order.wallet_id);

        assert_eq!(positions.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn positions_cache_get_by_wallet() {
        let position = new_position();
        let mut cache = PositionsCache::with_capacity(10);

        cache.add(position.clone());
        let order = position.get_order();
        let positions = cache.get_by_wallet_id(&order.wallet_id);

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
            created_date: DateTimeAsMicroseconds::now(),
            desire_price: None,
            funding_fee_period: None,
            invest_assets: HashMap::from([invest_asset]),
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
        let bidask = BidAsk {
            ask: 14.748,
            bid: 14.748,
            datetime: DateTimeAsMicroseconds::now(),
            instrument: "ATOMUSDT".to_string(),
        };

        order.open(&bidask, &prices)
    }
}
