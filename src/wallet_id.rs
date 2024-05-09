use std::fmt::Display;
use uuid::Uuid;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct WalletId(pub Uuid);

impl From<&str> for WalletId {
    fn from(value: &str) -> Self {
        WalletId(Uuid::parse_str(value).expect("invalid wallet id"))
    }
}

impl From<String> for WalletId {
    fn from(value: String) -> Self {
        WalletId(Uuid::parse_str(&value).expect("invalid wallet id"))
    }
}

impl From<Uuid> for WalletId {
    fn from(value: Uuid) -> Self {
        WalletId(value)
    }
}

impl Display for WalletId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}