use crate::positions::Position;
use std::collections::{BTreeMap, HashMap};
use tokio::sync::RwLock;

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
