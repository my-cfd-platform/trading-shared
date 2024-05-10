use std::fmt::Display;
use uuid::Uuid;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct WalletId(pub String);

impl From<&str> for WalletId {
    fn from(value: &str) -> Self {
        WalletId(value.to_string())
    }
}

impl From<&String> for WalletId {
    fn from(value: &String) -> Self {
        WalletId(value.to_owned())
    }
}

impl From<String> for WalletId {
    fn from(value: String) -> Self {
        WalletId(value)
    }
}

impl From<Uuid> for WalletId {
    fn from(value: Uuid) -> Self {
        WalletId(value.to_string())
    }
}

impl Display for WalletId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::wallet_id::WalletId;

    #[test]
    fn it_works() {
        let str = "g6a6c5ee305814ecdb98e9a2fa9c44123";
        let wallet_id: WalletId = str.into();
        println!("{}", wallet_id)
    }
}