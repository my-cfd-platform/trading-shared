use std::sync::Arc;
use crate::{
    caches::PositionsCache,
    positions::{BidAsk, ClosedPosition, Position},
};

pub struct PositionsMonitor {
    positions_cache: PositionsCache,
}

impl PositionsMonitor {
    pub fn new(positions_cache: PositionsCache) -> Self {
        Self {
            positions_cache,
        }
    }

    pub fn remove_position(&mut self, position_id: &str, wallet_id: &str) -> Option<Position> {
        return self.positions_cache.remove(position_id, wallet_id);
    }

    pub fn add_position(&mut self, position: Position) {
        self.positions_cache.add(position);
    }

    pub fn get_positions(&mut self, wallet_id: &str) -> Vec<Arc<Position>> {
        let positions = self.positions_cache.get_by_wallet_id(wallet_id);

        positions
    }

    pub fn update_positions(&mut self, bidask: &BidAsk) -> Vec<ClosedPosition> {
        let mut closed_positions = Vec::new();
        let positions = self.positions_cache.get_by_instrument(&bidask.instrument);

        for position in positions {
            let position =
            self.positions_cache.remove(&position.get_id(), &position.get_order().wallet_id);
            let position = position.unwrap();

            match position {
                Position::Closed(closed_position) => {
                    closed_positions.push(closed_position);
                }
                Position::Pending(mut pending_position) => {
                    pending_position.update(bidask);
                    let position = pending_position.try_activate();
                    self.positions_cache.add(position);
                }
                Position::Active(mut active_position) => {
                    active_position.update(bidask);
                    let position = active_position.try_close();
                    match position {
                        Position::Closed(closed_position) => closed_positions.push(closed_position),
                        Position::Active(position) => {
                            self.positions_cache.add(Position::Active(position))
                        }
                        Position::Pending(position) => {
                            self.positions_cache.add(Position::Pending(position))
                        }
                    }
                }
            }
        }

        closed_positions
    }
}
