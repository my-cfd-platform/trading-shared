use crate::positions::{OrderSide, Position, PositionBidAsk};
use std::{collections::HashMap, mem};

pub struct BidAsksCache {
    bidasks_by_instruments: HashMap<String, PositionBidAsk>,
}

impl BidAsksCache {
    pub fn new() -> Self {
        Self {
            bidasks_by_instruments: HashMap::with_capacity(200),
        }
    }

    pub fn update(&mut self, bidask: PositionBidAsk) {
        let current_bidask = self.bidasks_by_instruments.get_mut(&bidask.instrument);

        if let Some(current_bidask) = current_bidask {
            _ = mem::replace(current_bidask, bidask);
        } else {
            self.bidasks_by_instruments
                .insert(bidask.instrument.clone(), bidask);
        }
    }

    pub fn get_average_price(&self, instrument: &str) -> Option<f64> {
        let bidask = self.bidasks_by_instruments.get(instrument);

        if let Some(bidask) = bidask {
            let price = bidask.ask + bidask.bid / 2.0;

            return Some(price);
        }

        None
    }

    pub fn get_close_price(&self, instrument: &str, side: &OrderSide) -> Option<f64> {
        let bidask = self.bidasks_by_instruments.get(instrument);

        if let Some(bidask) = bidask {
            let close_price = bidask.get_close_price(side);

            return Some(close_price);
        }

        None
    }

    pub fn get_open_price(&self, instrument: &str, side: &OrderSide) -> Option<f64> {
        let bidask = self.bidasks_by_instruments.get(instrument);

        if let Some(bidask) = bidask {
            let close_price = bidask.get_open_price(side);

            return Some(close_price);
        }

        None
    }
}

pub struct PositionsCache {
    positions_by_wallets: HashMap<String, HashMap<String, Position>>,
}

impl PositionsCache {
    pub fn new() -> PositionsCache {
        PositionsCache {
            positions_by_wallets: HashMap::new(),
        }
    }

    pub fn add(&mut self, position: Position) {
        let wallet_positions = self
            .positions_by_wallets
            .get_mut(&position.get_order().wallet_id);
        let position_id = position.get_id();

        match wallet_positions {
            Some(positions) => {
                positions.insert(position_id.to_owned(), position);
            }
            None => {
                let wallet_id = position.get_order().wallet_id.clone();
                let positions_by_ids = HashMap::from([(position_id.to_owned(), position)]);
                self.positions_by_wallets
                    .insert(wallet_id, positions_by_ids);
            }
        }
    }

    pub fn get(&self) -> Vec<&Position> {
        let positions: Vec<&Position> = self
            .positions_by_wallets
            .values()
            .flat_map(|positions_by_ids| positions_by_ids.values())
            .collect();

        positions
    }

    pub fn find(&self, wallet_id: &str, position_id: &str) -> Option<&Position> {
        let wallet_positions = self.positions_by_wallets.get(wallet_id);

        if let Some(wallet_positions) = wallet_positions {
            let position = wallet_positions.get(position_id);

            return position;
        }

        None
    }

    pub fn remove(&mut self, position_id: &str, wallet_id: &str) -> Option<Position> {
        let wallet_positions = self
            .positions_by_wallets
            .get_mut(wallet_id);

        let position = match wallet_positions {
            Some(positions) => {
                let postion = positions.remove(position_id);

                postion
            }
            None => None,
        };

        position
    }
}
