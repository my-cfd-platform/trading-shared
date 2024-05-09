use ahash::AHashMap;
use compact_str::CompactString;
use rust_extensions::date_time::DateTimeAsMicroseconds;

#[derive(Debug, Clone)]
pub struct ActiveTopUp {
    pub id: String,
    pub date: DateTimeAsMicroseconds,
    pub total_assets: AHashMap<CompactString, f64>,
    pub instrument_price: f64,
    pub asset_prices: AHashMap<CompactString, f64>,
    pub bonus_assets: AHashMap<CompactString, f64>,
}

impl ActiveTopUp {
    pub fn cancel(self, instrument_price: f64) -> CanceledTopUp {
        CanceledTopUp {
            id: self.id,
            date: self.date,
            total_assets: self.total_assets,
            instrument_price: self.instrument_price,
            asset_prices: self.asset_prices,
            cancel_instrument_price: instrument_price,
            cancel_date: DateTimeAsMicroseconds::now(),
            bonus_assets: self.bonus_assets,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CanceledTopUp {
    pub id: String,
    pub date: DateTimeAsMicroseconds,
    pub total_assets: AHashMap<CompactString, f64>,
    pub instrument_price: f64,
    pub asset_prices: AHashMap<CompactString, f64>,
    pub cancel_instrument_price: f64,
    pub cancel_date: DateTimeAsMicroseconds,
    pub bonus_assets:AHashMap<CompactString, f64>,
}
