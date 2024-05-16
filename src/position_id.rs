use rust_extensions::sorted_vec::EntityWithKey;
use std::fmt::Display;
use uuid::Uuid;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct PositionId(pub Uuid);

impl EntityWithKey<PositionId> for PositionId {
    fn get_key(&self) -> &PositionId {
        self
    }
}

impl From<Uuid> for PositionId {
    fn from(value: Uuid) -> Self {
        PositionId(value)
    }
}

impl TryFrom<&str> for PositionId {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let uuid = Uuid::try_parse(value);

        let Ok(uuid) = uuid else {
            return Err(uuid.unwrap_err().to_string());
        };

        Ok(PositionId(uuid))
    }
}

impl TryFrom<&String> for PositionId {
    type Error = String;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        let uuid = Uuid::try_parse(value);

        let Ok(uuid) = uuid else {
            return Err(uuid.unwrap_err().to_string());
        };

        Ok(PositionId(uuid))
    }
}

impl TryFrom<String> for PositionId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let uuid = Uuid::try_parse(&value);

        let Ok(uuid) = uuid else {
            return Err(uuid.unwrap_err().to_string());
        };

        Ok(PositionId(uuid))
    }
}

impl Display for PositionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}
