use crate::error::DomainError;
use chrono::{DateTime, Utc};
use shared::domain::value_objects::OrderId;
use uuid::Uuid;

/// Lifecycle of the order processing saga orchestrated by the order service.
///
/// ```text
/// STARTED --(restaurant approved)--> PROCESSING --(payment completed)--> SUCCEEDED
///    |                                   |
///    +--(restaurant rejected)--> FAILED <-+--(payment failed)
/// ```
#[derive(PartialEq, Debug, Clone, Copy, Default)]
pub enum SagaStatus {
    #[default]
    Started,
    Processing,
    Succeeded,
    Failed,
}

impl SagaStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SagaStatus::Started => "STARTED",
            SagaStatus::Processing => "PROCESSING",
            SagaStatus::Succeeded => "SUCCEEDED",
            SagaStatus::Failed => "FAILED",
        }
    }
}

impl TryFrom<&str> for SagaStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "STARTED" => Ok(SagaStatus::Started),
            "PROCESSING" => Ok(SagaStatus::Processing),
            "SUCCEEDED" => Ok(SagaStatus::Succeeded),
            "FAILED" => Ok(SagaStatus::Failed),
            other => Err(DomainError::SagaError {
                message: format!("Unknown saga status: {other}"),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderSaga {
    id: Uuid,
    order_id: OrderId,
    status: SagaStatus,
    started_at: DateTime<Utc>,
}

impl OrderSaga {
    /// Starts a new saga for a freshly created order.
    pub fn start(order_id: OrderId) -> Self {
        Self {
            id: Uuid::new_v4(),
            order_id,
            status: SagaStatus::Started,
            started_at: Utc::now(),
        }
    }

    /// Rebuilds a saga from persisted state.
    pub fn restore(
        id: Uuid,
        order_id: OrderId,
        status: SagaStatus,
        started_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            order_id,
            status,
            started_at,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn order_id(&self) -> OrderId {
        self.order_id
    }

    pub fn status(&self) -> SagaStatus {
        self.status
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    /// The restaurant approved the order, the saga moves on to the payment step.
    pub fn restaurant_approved(&mut self) -> Result<(), DomainError> {
        self.transition(&[SagaStatus::Started], SagaStatus::Processing)
    }

    /// The payment completed, the saga is finished.
    pub fn payment_completed(&mut self) -> Result<(), DomainError> {
        self.transition(&[SagaStatus::Processing], SagaStatus::Succeeded)
    }

    /// A participant rejected the order, the saga ends unsuccessfully.
    pub fn fail(&mut self) -> Result<(), DomainError> {
        self.transition(
            &[SagaStatus::Started, SagaStatus::Processing],
            SagaStatus::Failed,
        )
    }

    fn transition(&mut self, from: &[SagaStatus], to: SagaStatus) -> Result<(), DomainError> {
        if !from.contains(&self.status) {
            return Err(DomainError::SagaError {
                message: format!(
                    "Saga {} cannot move from {} to {}",
                    self.id,
                    self.status.as_str(),
                    to.as_str(),
                ),
            });
        }

        self.status = to;
        Ok(())
    }
}
