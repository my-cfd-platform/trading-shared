use std::fmt::Display;
use uuid::Uuid;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct PositionId(pub Uuid);

impl From<&str> for PositionId {
    fn from(value: &str) -> Self {
        PositionId(Uuid::parse_str(value).expect("invalid position id"))
    }
}

impl From<Uuid> for PositionId {
    fn from(value: Uuid) -> Self {
        PositionId(value)
    }
}

impl From<String> for PositionId {
    fn from(value: String) -> Self {
        PositionId(Uuid::parse_str(&value).expect("invalid position id"))
    }
}

impl Display for PositionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}