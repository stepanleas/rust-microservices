use std::fmt;
use uuid::Uuid;

/// Declares a typed id wrapping a [`Uuid`], so ids of different entities cannot be mixed up.
macro_rules! typed_id {
    ($($name:ident),+ $(,)?) => {$(
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            #[allow(clippy::should_implement_trait)]
            pub fn from_str(s: &str) -> Result<Self, uuid::Error> {
                Ok(Self(Uuid::parse_str(s)?))
            }

            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    )+};
}

typed_id!(OrderId, OrderItemId, CustomerId, ProductId, CategoryId);
