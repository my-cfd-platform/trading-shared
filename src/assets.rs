use rust_extensions::sorted_vec::EntityWithKey;
use crate::asset_symbol::AssetSymbol;

#[derive(Clone, Debug)]
pub struct AssetAmount {
    pub amount: f64,
    pub symbol: AssetSymbol,
}

impl EntityWithKey<AssetSymbol> for AssetAmount {
    fn get_key(&self) -> &AssetSymbol {
        &self.symbol
    }
}

#[derive(Clone, Debug)]
pub struct AssetPrice {
    pub price: f64,
    pub symbol: AssetSymbol,
}

impl AssetPrice {
    pub fn new(symbol: AssetSymbol, price: f64) -> Self  {
        Self {
            price,
            symbol,
        }
    }
}

impl EntityWithKey<AssetSymbol> for AssetPrice {
    fn get_key(&self) -> &AssetSymbol {
        &self.symbol
    }
}

