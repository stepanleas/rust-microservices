use crate::enums::OrderApprovalStatus;
use crate::error::DomainError;
use crate::value_objects::OrderApprovalId;
use chrono::{DateTime, Utc};
use shared::domain::value_objects::OrderId;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct OrderApproval {
    id: OrderApprovalId,
    saga_id: Uuid,
    order_id: OrderId,
    status: OrderApprovalStatus,
    failure_messages: Vec<String>,
    created_at: DateTime<Utc>,
}

impl OrderApproval {
    pub fn new(
        id: OrderApprovalId,
        saga_id: Uuid,
        order_id: OrderId,
        status: OrderApprovalStatus,
        failure_messages: Vec<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            saga_id,
            order_id,
            status,
            failure_messages,
            created_at,
        }
    }

    pub fn approved(saga_id: Uuid, order_id: OrderId) -> Self {
        Self::new(
            OrderApprovalId::new(),
            saga_id,
            order_id,
            OrderApprovalStatus::Approved,
            vec![],
            Utc::now(),
        )
    }

    pub fn rejected(saga_id: Uuid, order_id: OrderId, failure_messages: Vec<String>) -> Self {
        Self::new(
            OrderApprovalId::new(),
            saga_id,
            order_id,
            OrderApprovalStatus::Rejected,
            failure_messages,
            Utc::now(),
        )
    }

    pub fn id(&self) -> OrderApprovalId {
        self.id
    }

    pub fn saga_id(&self) -> Uuid {
        self.saga_id
    }

    pub fn order_id(&self) -> OrderId {
        self.order_id
    }

    pub fn status(&self) -> OrderApprovalStatus {
        self.status
    }

    pub fn is_approved(&self) -> bool {
        self.status == OrderApprovalStatus::Approved
    }

    /// Compensation: the order was cancelled after the restaurant approved it.
    ///
    /// Only an approved order can be cancelled; the caller gives the stock back.
    pub fn cancel(&mut self) -> Result<(), DomainError> {
        if self.status != OrderApprovalStatus::Approved {
            return Err(DomainError::ApprovalNotCancellable {
                order_id: self.order_id.to_string(),
                status: self.status,
            });
        }

        self.status = OrderApprovalStatus::Cancelled;
        Ok(())
    }

    pub fn failure_messages(&self) -> &[String] {
        &self.failure_messages
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
