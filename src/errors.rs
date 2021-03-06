use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransactionProcessingError {
    ReusedTransactionId,
    AmountNotSpecified,
    NoSufficientFunds,
    UnknownTransactionId,
    DoubleDispute,
    DisputeNotActive,
    DisputeOnWithdrawal,
}

impl std::fmt::Display for TransactionProcessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
