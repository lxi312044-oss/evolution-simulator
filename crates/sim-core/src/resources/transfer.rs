#[derive(Debug, Clone, PartialEq)]
pub enum ResourceError {
    InvalidAccount,
    InvalidAmount,
    InsufficientBalance,
    CapacityExceeded,
    SameAccount,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransferReceipt {
    pub amount: f64,
}
