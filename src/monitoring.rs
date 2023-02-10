use crate::{
    caches::PositionsCache,
    positions::{ActivePosition, BidAsk, ClosedPosition, Position},
};
use std::sync::Arc;

pub struct PositionsMonitor {
    positions_cache: PositionsCache,
}

impl PositionsMonitor {
    pub fn new(positions_cache: PositionsCache) -> Self {
        Self { positions_cache }
    }

    pub fn remove(&mut self, position_id: &str, wallet_id: &str) -> Option<Position> {
        return self.positions_cache.remove(position_id, wallet_id);
    }

    pub fn add(&mut self, position: Position) {
        self.positions_cache.add(position);
    }

    pub fn get_by_wallet_id(&self, wallet_id: &str) -> Vec<Arc<Position>> {
        let positions = self.positions_cache.get_by_wallet_id(wallet_id);

        positions
    }

    pub fn update(&mut self, bidask: &BidAsk) -> Vec<PositionMonitoringEvent> {
        let mut events = Vec::new();
        let positions = self
            .positions_cache
            .remove_part_by_instrument(&bidask.instrument);

        if let Some(positions) = positions {
            for (id, position) in positions.take_all() {
                let position = self
                    .positions_cache
                    .remove(&id, &position.get_order().wallet_id);
                let position = position.expect("Must exists");

                match position {
                    Position::Closed(closed_position) => {
                        events.push(PositionMonitoringEvent::PositionClosed(closed_position));
                    }
                    Position::Pending(mut pending_position) => {
                        pending_position.update(bidask);
                        let position = pending_position.try_activate();

                        match position {
                            Position::Closed(_) => {
                                panic!("Pending position can't become Closed after try_activate")
                            }
                            Position::Active(position) => {
                                events.push(PositionMonitoringEvent::PositionActivated(
                                    position.clone(),
                                ));
                                self.positions_cache.add(Position::Active(position))
                            }
                            Position::Pending(position) => {
                                self.positions_cache.add(Position::Pending(position))
                            }
                        }
                    }
                    Position::Active(mut active_position) => {
                        active_position.update(bidask);
                        let position = active_position.try_close();

                        match position {
                            Position::Closed(closed_position) => events
                                .push(PositionMonitoringEvent::PositionClosed(closed_position)),
                            Position::Active(position) => {
                                self.positions_cache.add(Position::Active(position))
                            }
                            Position::Pending(_) => {
                                panic!("Active position can't become Pending")
                            }
                        }
                    }
                }
            }
        }

        events
    }
}

pub enum PositionMonitoringEvent {
    PositionClosed(ClosedPosition),
    PositionActivated(ActivePosition),
}

#[cfg(test)]
mod tests {}
