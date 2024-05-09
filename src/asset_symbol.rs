use std::fmt::Display;
use compact_str::CompactString;
use rust_extensions::sorted_vec::EntityWithKey;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct AssetSymbol(pub CompactString);

impl From<&str> for AssetSymbol {
    fn from(value: &str) -> Self {
       AssetSymbol(value.into())
    }
}

impl From<String> for AssetSymbol {
    fn from(value: String) -> Self {
        AssetSymbol(value.into())
    }
}

impl Display for AssetSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

impl EntityWithKey<AssetSymbol> for AssetSymbol {
    fn get_key(&self) -> &AssetSymbol {
        self
    }
}