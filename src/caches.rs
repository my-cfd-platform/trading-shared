use std::mem;
use crate::positions::{BidAsk, Position};
use ahash::{AHashMap, AHashSet};
use rust_extensions::sorted_vec::{EntityWithKey, SortedVec};
use crate::asset_symbol::AssetSymbol;
use crate::assets::AssetPrice;
use crate::instrument_symbol::InstrumentSymbol;
use crate::position_id::PositionId;
use crate::wallet_id::WalletId;

impl EntityWithKey<InstrumentSymbol> for BidAsk {
    fn get_key(&self) -> &InstrumentSymbol {
        &self.instrument
    }
}

#[derive(Clone, Debug)]
pub struct BidAsksCache {
    items: SortedVec<InstrumentSymbol, BidAsk>,
}

impl BidAsksCache {
    pub fn new(src: Vec<BidAsk>) -> Self {
        let mut items = SortedVec::new_with_capacity(src.len());

        for item in src.into_iter() {
            items.insert_or_replace(item);
        }

        Self {
            items,
        }
    }

    pub fn update(&mut self, bidask: BidAsk) {
        let current_bidask = self.items.get_mut(&bidask.instrument);

        if let Some(current_bidask) = current_bidask {
            _ = mem::replace(current_bidask, bidask);
        } else {
            self.items.insert_or_replace(bidask);
        }
    }

    pub fn get(&self, instrument: &InstrumentSymbol) -> Option<&BidAsk> {
        self.items.get(instrument)
    }

    pub fn find(&self, base_asset: &str, assets: &[&str]) -> SortedVec<InstrumentSymbol, BidAsk> {
        let mut bidasks = SortedVec::new_with_capacity(assets.len());
        let base_asset: AssetSymbol = base_asset.into();

        for asset in assets.iter() {
            let asset: AssetSymbol = (*asset).into();

            let instrument = BidAsk::get_instrument_symbol(&asset, &base_asset);
            let bidask = self.items.get(&instrument);

            if let Some(bidask) = bidask {
                bidasks.insert_or_replace(bidask.to_owned());
            }
        }

        bidasks
    }

    pub fn find_prices(&self, to_asset: &AssetSymbol, from_assets: &[&AssetSymbol]) -> SortedVec<AssetSymbol, AssetPrice> {
        let mut prices = SortedVec::new_with_capacity(from_assets.len());

        for asset in from_assets {
            let symbol = *asset;

            if *asset == to_asset {
                prices.insert_or_replace(AssetPrice {price: 1.0, symbol: symbol.clone()});
                continue;
            }

            let instrument = BidAsk::get_instrument_symbol(asset, to_asset);
            let bidask = self.items.get(&instrument);

            if let Some(bidask) = bidask {
                let price = bidask.get_asset_price(asset, &crate::orders::OrderSide::Sell);
                prices.insert_or_replace(AssetPrice {price, symbol: symbol.clone()});
            }
        }

        prices
    }
}

pub struct PositionsCache {
    positions_by_ids: AHashMap<PositionId, Position>,
    ids_by_wallets: AHashMap<WalletId, AHashSet<PositionId>>,
}

impl PositionsCache {
    pub fn with_capacity(capacity: usize) -> PositionsCache {
        PositionsCache {
            ids_by_wallets: AHashMap::with_capacity(capacity),
            positions_by_ids: AHashMap::with_capacity(capacity),
        }
    }
    
    pub fn count(&self) -> usize {
        self.positions_by_ids.len()
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

    pub fn get_by_wallet_id(&self, wallet_id: &WalletId) -> Vec<&Position> {
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

    pub fn contains_by_wallet_id(&self, wallet_id: &WalletId) -> bool {
        self.ids_by_wallets.contains_key(wallet_id)
    }

    pub fn get_mut(&mut self, id: &PositionId) -> Option<&mut Position> {
        self.positions_by_ids.get_mut(id)
    }

    pub fn remove(&mut self, position_id: &PositionId) -> Option<Position> {
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
    use super::{PositionsCache};
    use crate::{
        orders::Order,
        positions::{BidAsk, Position},
    };
    use rust_extensions::sorted_vec::SortedVec;
    use uuid::Uuid;
    use crate::assets::{AssetAmount, AssetPrice};

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
        let mut invest_assets = SortedVec::new();
        invest_assets.insert_or_replace(AssetAmount {amount: 100.0, symbol: "BTC".into()});
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
            side: crate::orders::OrderSide::Buy,
            take_profit: None,
            stop_loss: None,
            stop_out_percent: 10.0,
            margin_call_percent: 10.0,
            top_up_enabled: false,
            top_up_percent: 10.0,
        };
        let mut prices = SortedVec::new();
        prices.insert_or_replace(AssetPrice {price: 22300.0, symbol: "BTC".into()});
        let bidask = BidAsk {
            ask: 14.748,
            bid: 14.748,
            datetime: DateTimeAsMicroseconds::now(),
            instrument: "ATOMUSDT".into(),
        };

        order.open(&bidask, &prices)
    }
}
