#[derive(PartialEq, Debug, Clone, Copy, Default)]
pub enum OrderStatus {
    #[default]
    Pending,
    Paid,
    Approved,
    Cancelling,
    Canceled,
}
