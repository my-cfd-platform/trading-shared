use crate::{
    caches::PositionsCache,
    positions::{ActivePosition, BidAsk, ClosedPosition, Position},
};
use ahash::{AHashMap, AHashSet};

pub struct PositionsMonitor {
    positions_cache: PositionsCache,
    ids_by_instruments: AHashMap<String, AHashSet<String>>,
}

impl PositionsMonitor {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            positions_cache: PositionsCache::with_capacity(capacity),
            ids_by_instruments: AHashMap::with_capacity(capacity),
        }
    }

    pub fn remove(&mut self, position_id: &str) -> Option<Position> {
        let position = self.positions_cache.remove(position_id);

        if let Some(position) = position.as_ref() {
            self.remove_from_instruments_map(position);
        }

        position
    }

    fn remove_from_instruments_map(&mut self, position: &Position) {
        for invest_instrument in position.get_order().get_instruments() {
            if let Some(ids) = self.ids_by_instruments.get_mut(&invest_instrument) {
                ids.remove(position.get_id());
            }
        }
    }

    pub fn add(&mut self, position: Position) {
        self.add_to_instruments_map(&position);
        self.positions_cache.add(position);
    }

    fn add_to_instruments_map(&mut self, position: &Position) {
        let id = position.get_id().to_owned();
        let invest_instruments = position.get_order().get_instruments();

        for invest_instrument in invest_instruments {
            if let Some(ids) = self.ids_by_instruments.get_mut(&invest_instrument) {
                ids.insert(id.clone());
            } else {
                self.ids_by_instruments
                    .insert(invest_instrument, AHashSet::from([id.clone()]));
            }
        }
    }

    pub fn get_by_wallet_id(&self, wallet_id: &str) -> Vec<&Position> {
        self.positions_cache.get_by_wallet_id(wallet_id)
    }

    pub fn update(&mut self, bidask: &BidAsk) -> Vec<PositionMonitoringEvent> {
        let ids = self.ids_by_instruments.get_mut(&bidask.instrument);

        let Some(ids) = ids else {
            return Vec::with_capacity(0);
        };

        let mut events = Vec::with_capacity(ids.len());

        ids.retain(|id| {
            let position = self.positions_cache.get_mut(id);

            let Some(position) = position else {
                return false;
            };

            match position {
                Position::Closed(_) => {
                    let position = match self.positions_cache.remove(id).expect("Checked") {
                        Position::Closed(position) => position,
                        _ => panic!("Checked"),
                    };
                    events.push(PositionMonitoringEvent::PositionClosed(position));

                    false
                }
                Position::Pending(position) => {
                    position.update(bidask);

                    if position.can_activate() {
                        let position = match self.positions_cache.remove(id).expect("Checked") {
                            Position::Pending(position) => position,
                            _ => panic!("Checked"),
                        };
                        let position = position.into_active();
                        events
                            .push(PositionMonitoringEvent::PositionActivated(position.clone()));
                        self.positions_cache.add(Position::Active(position));
                    }

                    true
                }
                Position::Active(position) => {
                    position.update(bidask);

                    if let Some(reason) = position.determine_close_reason() {
                        let position =
                            match self.positions_cache.remove(id).expect("Must exists") {
                                Position::Active(position) => position,
                                _ => panic!("Checked"),
                            };
                        let position = position.close(reason);
                        events.push(PositionMonitoringEvent::PositionClosed(position));

                        false
                    } else {
                        true
                    }
                }
            }
        });

        events
    }
}

pub enum PositionMonitoringEvent {
    PositionClosed(ClosedPosition),
    PositionActivated(ActivePosition),
}

#[cfg(test)]
mod tests {}
