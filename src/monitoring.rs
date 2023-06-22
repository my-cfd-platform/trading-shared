use std::time::Duration;
use crate::{
    caches::PositionsCache,
    positions::{ActivePosition, BidAsk, ClosedPosition, Position},
};
use ahash::{AHashMap, AHashSet};
use crate::top_ups::{CanceledTopUp};

pub struct PositionsMonitor {
    positions_cache: PositionsCache,
    ids_by_instruments: AHashMap<String, AHashSet<String>>,
    cancel_top_up_delay: Duration
}

impl PositionsMonitor {
    pub fn with_capacity(capacity: usize, cancel_top_up_delay: Duration) -> Self {
        Self {
            positions_cache: PositionsCache::with_capacity(capacity),
            ids_by_instruments: AHashMap::with_capacity(capacity),
            cancel_top_up_delay,
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
        for instrument in position.get_instruments() {
            if let Some(ids) = self.ids_by_instruments.get_mut(&instrument) {
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
        let instruments = position.get_instruments();

        for invest_instrument in instruments {
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
        let position_ids = self.ids_by_instruments.get_mut(&bidask.instrument);

        let Some(position_ids) = position_ids else {
            return Vec::with_capacity(0);
        };

        let mut events = Vec::with_capacity(position_ids.len());

        position_ids.retain(|position_id| {
            let position = self.positions_cache.get_mut(position_id);

            let Some(position) = position else {
                return false; // no position in cache so remove id from instruments map
            };

            match position {
                Position::Closed(_) => {
                    let position = match self.positions_cache.remove(position_id).expect("Checked") {
                        Position::Closed(position) => position,
                        _ => panic!("Checked"),
                    };
                    events.push(PositionMonitoringEvent::PositionClosed(position));

                    false // remove closed position
                }
                Position::Pending(position) => {
                    position.update(bidask);

                    if position.can_activate() {
                        let position = match self.positions_cache.remove(position_id).expect("Checked") {
                            Position::Pending(position) => position,
                            _ => panic!("Checked"),
                        };
                        let position = position.into_active();
                        events
                            .push(PositionMonitoringEvent::PositionActivated(position.clone()));
                        self.positions_cache.add(Position::Active(position));
                    }

                    true // active position must be monitored
                }
                Position::Active(position) => {
                    position.update(bidask);

                    if position.is_margin_call() {
                        events.push(PositionMonitoringEvent::PositionMarginCall(position.clone()));
                    }

                    if position.is_top_up() {
                        let position =
                            match self.positions_cache.remove(position_id).expect("Must exists") {
                                Position::Active(position) => position,
                                _ => panic!("Position is in Active case"),
                            };
                        events.push(PositionMonitoringEvent::PositionTopUp(position));

                        return false; // top-up required for position
                    } else {
                        let canceled_top_ups = position.try_cancel_top_ups(self.cancel_top_up_delay);

                        if !canceled_top_ups.is_empty() {
                            let position =
                                match self.positions_cache.remove(position_id).expect("Must exists") {
                                    Position::Active(position) => position,
                                    _ => panic!("Position is in Active case"),
                                };
                            events.push(PositionMonitoringEvent::PositionTopUpCanceled((position, canceled_top_ups)));

                            return false; // top-up cancel required for position
                        }
                    }

                    if let Some(reason) = position.determine_close_reason() {
                        let position =
                            match self.positions_cache.remove(position_id).expect("Must exists") {
                                Position::Active(position) => position,
                                _ => panic!("Position is in Active case"),
                            };
                        let position = position.close(reason);
                        events.push(PositionMonitoringEvent::PositionClosed(position));

                        false // remove closed position
                    } else {
                        true // no need to do anything with position
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
    PositionMarginCall(ActivePosition),
    PositionTopUp(ActivePosition),
    PositionTopUpCanceled((ActivePosition, Vec<CanceledTopUp>)),
}

#[cfg(test)]
mod tests {}
