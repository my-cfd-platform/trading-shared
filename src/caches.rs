use std::{collections::{BTreeMap, HashMap}};
use tokio::sync::RwLock;
use crate::positions::Position;

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
            Position::Active(_state, order) => {
                let mut positions_by_instruments = self.positions_by_instruments.write().await;
                let positions = positions_by_instruments.get_mut(&order.instument);

                match positions {
                    Some(positions) => {
                        positions.insert(order.id.clone(), position);
                    },
                    None => {
                        let instrument = order.instument.clone();
                        let positions_by_ids = HashMap::from([(order.id.clone(), position)]);
                        positions_by_instruments.insert(instrument, positions_by_ids);
                    }
                }
            }
            // todo: support all types
            Position::Closed(_, _) => panic!("Closed position can't be added to cache"),
            Position::Pending(_, _) => panic!("Pending position can't be added to cache"),
        }
    }
}