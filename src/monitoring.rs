use crate::positions::PendingPosition;
use crate::top_ups::{ActiveTopUp, CanceledTopUp};
use crate::wallets::{Wallet, WalletBalance};
use crate::{
    caches::PositionsCache,
    positions::{ActivePosition, BidAsk, ClosedPosition, Position},
};
use ahash::{AHashMap, AHashSet};
use std::time::Duration;
use compact_str::CompactString;

pub struct PositionsMonitor {
    positions_cache: PositionsCache,
    ids_by_instruments: AHashMap<CompactString, AHashSet<String>>,
    cancel_top_up_delay: Duration,
    cancel_top_up_price_change_percent: f64,
    locked_ids: AHashSet<String>,
    pnl_accuracy: Option<u32>,
    wallets_by_ids: AHashMap<String, Wallet>,
    wallet_ids_by_instruments: AHashMap<CompactString, AHashSet<String>>,
}

impl PositionsMonitor {
    pub fn new(
        capacity: usize,
        cancel_top_up_delay: Duration,
        cancel_top_up_price_change_percent: f64,
        pnl_accuracy: Option<u32>,
    ) -> Self {
        Self {
            wallets_by_ids: Default::default(),
            positions_cache: PositionsCache::with_capacity(capacity),
            ids_by_instruments: AHashMap::with_capacity(capacity),
            cancel_top_up_delay,
            locked_ids: AHashSet::with_capacity(capacity),
            cancel_top_up_price_change_percent,
            pnl_accuracy,
            wallet_ids_by_instruments: Default::default(),
        }
    }

    pub fn get_wallet_mut(&mut self, wallet_id: &str) -> Option<&mut Wallet> {
        let wallet = self.wallets_by_ids.get_mut(wallet_id);

        if let Some(wallet) = wallet {
            return Some(wallet);
        }

        None
    }

    pub fn contains_wallet(&self, wallet_id: &str) -> bool {
        self.wallets_by_ids.contains_key(wallet_id)
    }

    pub fn remove(&mut self, position_id: &str) -> Option<Position> {
        if self.locked_ids.contains(position_id) {
            return None;
        }

        let position = self.positions_cache.remove(position_id);

        if let Some(position) = position.as_ref() {
            match position {
                Position::Active(position) => {
                    if position.order.top_up_enabled
                        && self
                            .positions_cache
                            .contains_by_wallet_id(&position.order.wallet_id)
                    {
                        let wallet = self.wallets_by_ids.get_mut(&position.order.wallet_id);

                        if let Some(wallet) = wallet {
                            wallet.deduct_top_up_pnl(
                                &position.order.instrument,
                                position.current_pnl,
                            );
                        }
                    } else {
                        self.remove_wallet(&position.order.wallet_id);
                    }
                }
                Position::Closed(_) => {}
                Position::Pending(_) => {}
            }

            for instrument in position.get_instruments() {
                if let Some(ids) = self.ids_by_instruments.get_mut(&instrument) {
                    ids.remove(position.get_id());
                }
            }
        }

        position
    }

    pub fn remove_wallet(&mut self, wallet_id: &str) -> Option<Wallet> {
        let wallet = self.wallets_by_ids.remove(wallet_id);

        if let Some(wallet) = wallet {
            for instrument in wallet.get_instruments() {
                let wallet_ids = self.wallet_ids_by_instruments.get_mut(instrument);

                if let Some(wallet_ids) = wallet_ids {
                    wallet_ids.remove(wallet_id);
                }
            }

            return Some(wallet);
        }

        None
    }

    pub fn add_wallet(&mut self, wallet: Wallet) {
        for instrument in wallet.get_instruments() {
            let wallet_ids = self.wallet_ids_by_instruments.get_mut(instrument);

            if let Some(wallet_ids) = wallet_ids {
                wallet_ids.insert(wallet.id.clone());
            } else {
                self.wallet_ids_by_instruments
                    .insert(instrument.to_owned(), AHashSet::from([wallet.id.clone()]));
            }
        }

        self.wallets_by_ids.insert(wallet.id.clone(), wallet);
    }

    pub fn update_wallet(
        &mut self,
        wallet_id: &str,
        balance: WalletBalance,
    ) -> Result<Option<Wallet>, String> {
        let wallet = self.wallets_by_ids.get_mut(wallet_id);

        let Some(wallet) = wallet else {
            return Ok(None);
        };

        wallet.update_balance(balance)?;

        Ok(Some(wallet.to_owned()))
    }

    pub fn add(&mut self, position: Position) {
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

        self.positions_cache.add(position);
    }

    pub fn get_by_wallet_id(&self, wallet_id: &str) -> Vec<&Position> {
        self.positions_cache.get_by_wallet_id(wallet_id)
    }

    pub fn unlock(&mut self, position_id: &str) {
        self.locked_ids.remove(position_id);
    }

    pub fn add_top_up(
        &mut self,
        position: &ActivePosition,
        top_up: ActiveTopUp,
    ) -> Result<(), String> {
        let position = self.positions_cache.get_mut(&position.id);

        let Some(position) = position else {
            return Err("Position not found".to_string());
        };

        match position {
            Position::Active(position) => {
                position.add_top_up(top_up);
                Ok(())
            }
            Position::Closed(_) => Err("Can't add top-up to closed position ".to_string()),
            Position::Pending(_) => Err("Can't add top-up to pending position".to_string()),
        }
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Position> {
        self.positions_cache.get_mut(id)
    }

    pub fn update(&mut self, bidask: &BidAsk) -> Vec<PositionMonitoringEvent> {
        let position_ids = self.ids_by_instruments.get_mut(&bidask.instrument);

        let Some(position_ids) = position_ids else {
            return Vec::with_capacity(0);
        };

        let mut events = Vec::with_capacity(position_ids.len());
        let mut top_up_pnls_by_wallet_ids: AHashMap<String, f64> =
            AHashMap::with_capacity(position_ids.len() / 2);
        let mut wallet_ids_to_remove = Vec::with_capacity(position_ids.len() / 3);
        let mut top_up_reserved_by_wallet_ids: AHashMap<String, AHashMap<CompactString, f64>> =
            AHashMap::with_capacity(position_ids.len() / 2);

        position_ids.retain(|position_id| {
            if self.locked_ids.contains(position_id) {
                // skip update
                return true;
            }

            let position = self.positions_cache.get_mut(position_id);

            let Some(position) = position else {
                return false; // no position in cache so remove id from instruments map
            };

            match position {
                Position::Closed(_) => {
                    let position = match self.positions_cache.remove(position_id).expect("Checked")
                    {
                        Position::Closed(position) => position,
                        _ => panic!("Checked"),
                    };
                    events.push(PositionMonitoringEvent::PositionClosed(position));

                    false // remove closed position
                }
                Position::Pending(position) => {
                    position.update(bidask);

                    if position.is_price_reached() {
                        if position.can_activate() {
                            let position =
                                match self.positions_cache.remove(position_id).expect("Checked") {
                                    Position::Pending(position) => position,
                                    _ => panic!("Checked"),
                                };
                            let mut position =
                                position.activate().expect("checked by can_activate");
                            position.update(bidask);
                            events
                                .push(PositionMonitoringEvent::PositionActivated(position.clone()));
                            self.positions_cache.add(Position::Active(position));
                        } else {
                            self.locked_ids.insert(position.id.clone());
                            let lock_reason =
                                PositionLockReason::ActivationPending(position.clone());
                            events.push(PositionMonitoringEvent::PositionLocked(lock_reason));
                        }
                    }

                    true // pending position must be monitored
                }
                Position::Active(position) => {
                    position.update(bidask);

                    if position.is_margin_call() {
                        events.push(PositionMonitoringEvent::PositionMarginCall(
                            position.clone(),
                        ));
                    }

                    if position.is_top_up() {
                        self.locked_ids.insert(position.id.clone());
                        let event = PositionMonitoringEvent::PositionLocked(
                            PositionLockReason::TopUp(position.to_owned()),
                        );
                        events.push(event);
                    } else {
                        let canceled_top_ups = position.try_cancel_top_ups(
                            self.cancel_top_up_price_change_percent,
                            self.cancel_top_up_delay,
                        );

                        if !canceled_top_ups.is_empty() {
                            self.locked_ids.insert(position.id.clone());
                            let reason = PositionLockReason::TopUpsCanceled((
                                position.to_owned(),
                                canceled_top_ups,
                            ));
                            let event = PositionMonitoringEvent::PositionLocked(reason);
                            events.push(event);
                        }
                    }

                    if let Some(reason) = position.determine_close_reason() {
                        let position = match self
                            .positions_cache
                            .remove(position_id)
                            .expect("Must exists")
                        {
                            Position::Active(position) => position,
                            _ => panic!("Position is in Active case"),
                        };
                        let position = position.close(reason, self.pnl_accuracy);

                        if self
                            .positions_cache
                            .contains_by_wallet_id(&position.order.wallet_id)
                        {
                            wallet_ids_to_remove.push(position.order.wallet_id.clone());
                        }

                        events.push(PositionMonitoringEvent::PositionClosed(position));

                        false // remove closed position
                    } else {
                        if position.order.top_up_enabled {
                            let wallet_pnl =
                                top_up_pnls_by_wallet_ids.get_mut(&position.order.wallet_id);

                            if let Some(wallet_pnl) = wallet_pnl {
                                *wallet_pnl += position.current_pnl;
                            } else {
                                top_up_pnls_by_wallet_ids
                                    .insert(position.order.wallet_id.clone(), position.current_pnl);
                            }

                            // calc reserved amounts
                            let reserved_by_assets =
                                top_up_reserved_by_wallet_ids.get_mut(&position.order.wallet_id);

                            if let Some(reserved_by_assets) = reserved_by_assets {
                                for (asset_symbol, asset_amount) in
                                    position.total_invest_assets.iter()
                                {
                                    let reserved_amount = reserved_by_assets.get_mut(asset_symbol);

                                    if let Some(reserved_amount) = reserved_amount {
                                        *reserved_amount += asset_amount;
                                    } else {
                                        reserved_by_assets
                                            .insert(asset_symbol.to_owned(), *asset_amount);
                                    }
                                }
                            } else {
                                top_up_reserved_by_wallet_ids.insert(
                                    position.order.wallet_id.clone(),
                                    position.order.invest_assets.clone(),
                                );
                            }
                        }

                        true // no need to do anything with position
                    }
                }
            }
        });

        for wallet_id in wallet_ids_to_remove {
            self.remove_wallet(&wallet_id);
        }

        self.update_wallet_prices(bidask);
        self.update_wallet_reserved(bidask, &top_up_reserved_by_wallet_ids);
        let wallet_events = self.update_wallet_pnls(bidask, top_up_pnls_by_wallet_ids);

        for event in wallet_events.into_iter() {
            events.push(event);
        }

        events
    }

    fn update_wallet_prices(&mut self, bidask: &BidAsk) {
        let wallet_ids = self.wallet_ids_by_instruments.get_mut(&bidask.instrument);

        if let Some(wallet_ids) = wallet_ids {
            for wallet_id in wallet_ids.iter() {
                let wallet = self
                    .wallets_by_ids
                    .get_mut(wallet_id)
                    .expect("invalid wallet add");
                wallet.update_price(bidask);
            }
        }
    }

    fn update_wallet_reserved(
        &mut self,
        bidask: &BidAsk,
        reserved_by_wallet_ids: &AHashMap<String, AHashMap<CompactString, f64>>,
    ) {
        for (wallet_id, reserved_by_assets) in reserved_by_wallet_ids {
            let wallet = self.wallets_by_ids.get_mut(wallet_id);

            let Some(wallet) = wallet else {
                continue;
            };

            wallet.set_top_up_reserved(&bidask.instrument, reserved_by_assets);
        }
    }

    fn update_wallet_pnls(
        &mut self,
        bidask: &BidAsk,
        pnls_by_wallet_ids: AHashMap<String, f64>,
    ) -> Vec<PositionMonitoringEvent> {
        let mut events = Vec::new();

        for (wallet_id, pnl) in pnls_by_wallet_ids {
            let wallet = self.wallets_by_ids.get_mut(&wallet_id);

            let Some(wallet) = wallet else {
                continue;
            };

            wallet.set_top_up_pnl(&bidask.instrument, pnl);
            wallet.update_loss();

            if wallet.is_margin_call() {
                events.push(PositionMonitoringEvent::WalletMarginCall(
                    WalletMarginCallInfo {
                        loss_percent: wallet.current_loss_percent,
                        pnl,
                        wallet_id: wallet.id.clone(),
                        trader_id: wallet.trader_id.clone(),
                    },
                ));
            }
        }

        events
    }
}

pub enum PositionMonitoringEvent {
    /// Active position was closed due to stop-out and removed from cache
    PositionClosed(ClosedPosition),
    /// Pending position with already reserved assets was activated due to price
    /// and re-added as active position to cache
    PositionActivated(ActivePosition),
    /// Active position has margin call
    PositionMarginCall(ActivePosition),
    /// Active position was locked with inner reason
    PositionLocked(PositionLockReason),
    /// Wallet has margin call
    WalletMarginCall(WalletMarginCallInfo),
}

pub enum PositionLockReason {
    /// Active position needs to add a top-up
    TopUp(ActivePosition),
    /// Active position needs to cancel the top-ups
    TopUpsCanceled((ActivePosition, Vec<CanceledTopUp>)),
    /// Pending position without reserved assets reached desire price needs to reserve assets
    ActivationPending(PendingPosition),
}

#[derive(Debug)]
pub struct WalletMarginCallInfo {
    pub loss_percent: f64,
    pub pnl: f64,
    pub wallet_id: String,
    pub trader_id: String,
}

#[cfg(test)]
mod tests {}
