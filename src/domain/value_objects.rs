use bigdecimal::BigDecimal;
use num_traits::{FromPrimitive, ToPrimitive};
use std::fmt;
use std::ops::{Add, Mul, Sub};
use uuid::Uuid;

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct OrderId(Uuid);

impl OrderId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn from_str(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Into<Uuid> for OrderId {
    fn into(self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct OrderItemId(Uuid);

impl OrderItemId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn from_str(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Into<Uuid> for OrderItemId {
    fn into(self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct CustomerId(Uuid);

impl CustomerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn from_str(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Into<Uuid> for CustomerId {
    fn into(self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct ProductId(Uuid);

impl ProductId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn from_str(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Into<Uuid> for ProductId {
    fn into(self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct CategoryId(Uuid);

impl CategoryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn from_str(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Into<Uuid> for CategoryId {
    fn into(self) -> Uuid {
        self.0
    }
}

