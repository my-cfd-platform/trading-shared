use crate::positions::{OrderSide, Position, PositionBidAsk};
use std::{
    collections::{BTreeMap, HashMap},
    mem,
};
use tokio::sync::RwLock;

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
    positions_by_instruments: RwLock<BTreeMap<String, HashMap<String, Position>>>,
}

impl PositionsCache {
    pub fn new() -> PositionsCache {
        PositionsCache {
            positions_by_instruments: RwLock::new(BTreeMap::new()),
        }
    }

    pub async fn add(&self, position: Position) {
        match &position {
            Position::Opened(opened_position) => {
                let mut positions_by_instruments = self.positions_by_instruments.write().await;
                let instrument = opened_position.order.instument.clone();
                let positions = positions_by_instruments.get_mut(&instrument);

                match positions {
                    Some(positions) => {
                        positions.insert(opened_position.id.clone(), position);
                    }
                    None => {
                        let positions_by_ids =
                            HashMap::from([(opened_position.id.clone(), position)]);
                        positions_by_instruments.insert(instrument, positions_by_ids);
                    }
                }
            }
            // todo: support all types
            Position::Closed(_) => panic!("Closed position can't be added to cache"),
            Position::Pending(_) => panic!("Pending position can't be added to cache"),
        }
    }
}
