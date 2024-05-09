use std::fmt::Display;
use compact_str::CompactString;
use rust_extensions::sorted_vec::EntityWithKey;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct InstrumentSymbol(pub CompactString);

impl From<&str> for InstrumentSymbol {
    fn from(value: &str) -> Self {
        InstrumentSymbol(value.into())
    }
}

impl From<String> for InstrumentSymbol {
    fn from(value: String) -> Self {
        InstrumentSymbol(value.into())
    }
}

impl Display for InstrumentSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

impl EntityWithKey<InstrumentSymbol> for InstrumentSymbol {
    fn get_key(&self) -> &InstrumentSymbol {
        self
    }
}