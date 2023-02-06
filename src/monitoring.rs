use std::rc::Rc;
use crate::{
    caches::PositionsCache,
    positions::{BidAsk, ClosedPosition, Position},
};
use tokio::sync::RwLock;

pub struct PositionsMonitor {
    positions_cache: RwLock<PositionsCache>,
}

impl PositionsMonitor {
    pub fn new(positions_cache: PositionsCache) -> Self {
        Self {
            positions_cache: RwLock::new(positions_cache),
        }
    }

    pub async fn add_position(&mut self, position: Position) {
        let mut cache = self.positions_cache.write().await;
        cache.add(position);
    }

    pub async fn get_positions(&mut self, wallet_id: &str) -> Vec<Rc<Position>> {
        let cache = self.positions_cache.read().await;
        let positions = cache.get_by_wallet_id(wallet_id);

        positions
    }

    pub async fn update_positions(&self, bidask: &BidAsk) -> Vec<ClosedPosition> {
        let mut closed_positions = Vec::new();
        let mut cache = self.positions_cache.write().await;
        let positions = cache.get_by_instrument(&bidask.instrument);

        for position in positions {
            let position =
                cache.remove(&position.get_id(), &position.get_order().wallet_id);
            let position = position.unwrap();

            match position {
                Position::Closed(closed_position) => {
                    closed_positions.push(closed_position);
                }
                Position::Pending(mut pending_position) => {
                    pending_position.update(bidask);
                    let position = pending_position.try_activate();
                    cache.add(position);
                }
                Position::Active(mut active_position) => {
                    active_position.update(bidask);
                    let position = active_position.try_close();
                    match position {
                        Position::Closed(closed_position) => closed_positions.push(closed_position),
                        Position::Active(position) => {
                            cache.add(Position::Active(position))
                        }
                        Position::Pending(position) => {
                            cache.add(Position::Pending(position))
                        }
                    }
                }
            }
        }

        closed_positions
    }
}
