use crate::components::reservation::domain::DomainError;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ApplicationError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("The room is already reserved for the specified time slot")]
    ReservationConflict,
    #[error("Reservation was not found")]
    ReservationNotFound,
    #[error("Could not resolve the civil day in the given timezone")]
    InvalidScheduleWindow,
    #[error("Infrastructure failure")]
    Infrastructure,
}
