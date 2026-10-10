mod order_approval;
mod product;

pub use order_approval::PostgresOrderApprovalRepository;
pub use product::PostgresProductRepository;

/// The outbox repository is shared by every service; only the message kinds differ.
pub type PostgresOutboxRepository =
    shared::outbox::postgres::PostgresOutboxRepository<application::outbox::OutboxMessageType>;
