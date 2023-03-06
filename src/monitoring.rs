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
            if let Some(keys) = self.ids_by_instruments.get_mut(&invest_instrument) {
                keys.insert(id.clone());
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

        if let Some(ids) = ids {
            let mut events = Vec::with_capacity(ids.len());

            ids.retain(|id| {
                let position = self.positions_cache.remove(id);
                let mut is_in_cache = false;

                if let Some(position) = position {
                    match position {
                        Position::Closed(closed_position) => {
                            events.push(PositionMonitoringEvent::PositionClosed(
                                closed_position,
                            ));
                            is_in_cache = false;
                        }
                        Position::Pending(mut pending_position) => {
                            pending_position.update(bidask);
                            let position = pending_position.try_activate();

                            match position {
                                Position::Closed(_) => {
                                    panic!(
                                        "Pending position can't become Closed after try_activate"
                                    )
                                }
                                Position::Active(position) => {
                                    events.push(PositionMonitoringEvent::PositionActivated(
                                        position.clone(),
                                    ));
                                    self.positions_cache.add(Position::Active(position));
                                    is_in_cache = true;
                                }
                                Position::Pending(position) => {
                                    self.positions_cache.add(Position::Pending(position));
                                    is_in_cache = true;
                                }
                            }
                        }
                        Position::Active(mut active_position) => {
                            active_position.update(bidask);
                            let position = active_position.try_close();

                            match position {
                                Position::Closed(closed_position) => {
                                    events.push(PositionMonitoringEvent::PositionClosed(
                                        closed_position,
                                    ));
                                    is_in_cache = true;
                                }
                                Position::Active(position) => {
                                    self.positions_cache.add(Position::Active(position));
                                    is_in_cache = true;
                                }
                                Position::Pending(_) => {
                                    panic!("Active position can't become Pending")
                                }
                            }
                        }
                    }
                }

                is_in_cache
            });
        }

        Vec::with_capacity(0)
    }
}

pub enum PositionMonitoringEvent {
    PositionClosed(ClosedPosition),
    PositionActivated(ActivePosition),
}

#[cfg(test)]
mod tests {

}
