use rust_extensions::date_time::DateTimeAsMicroseconds;
use rust_extensions::sorted_vec::SortedVec;
use crate::asset_symbol::AssetSymbol;
use crate::assets::{AssetAmount, AssetPrice};

#[derive(Debug, Clone)]
pub struct ActiveTopUp {
    pub id: String,
    pub date: DateTimeAsMicroseconds,
    pub total_assets: SortedVec<AssetSymbol, AssetAmount>,
    pub instrument_price: f64,
    pub asset_prices: SortedVec<AssetSymbol, AssetPrice>,
    pub bonus_assets: SortedVec<AssetSymbol, AssetAmount>,
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
    pub total_assets: SortedVec<AssetSymbol, AssetAmount>,
    pub instrument_price: f64,
    pub asset_prices: SortedVec<AssetSymbol, AssetPrice>,
    pub cancel_instrument_price: f64,
    pub cancel_date: DateTimeAsMicroseconds,
    pub bonus_assets:SortedVec<AssetSymbol, AssetAmount>,
}
