use std::fmt;
use uuid::Uuid;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct OrderApprovalId(Uuid);

impl OrderApprovalId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl From<OrderApprovalId> for Uuid {
    fn from(value: OrderApprovalId) -> Self {
        value.0
    }
}

impl fmt::Display for OrderApprovalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
