use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("Start date must be earlier than end date")]
    InvalidTimeRange,
    #[error("Reservation is already cancelled")]
    AlreadyCancelled,
    #[error("Unknown reservation status: {0}")]
    InvalidStatus(String),
}
