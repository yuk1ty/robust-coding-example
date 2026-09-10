use chrono::{DateTime, Utc};

use super::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingTimeRange {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl MeetingTimeRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Self, DomainError> {
        if start >= end {
            return Err(DomainError::InvalidTimeRange);
        }
        Ok(Self { start, end })
    }

    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }

    pub fn end(&self) -> DateTime<Utc> {
        self.end
    }

    pub fn overlaps(&self, other: &MeetingTimeRange) -> bool {
        self.start < other.end && other.start < self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationStatus {
    Confirmed,
    Cancelled,
}

impl ReservationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "confirmed" => Ok(Self::Confirmed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(DomainError::InvalidStatus(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationFee {
    amount_minor: i64,
    currency: String,
}

impl CancellationFee {
    pub fn new(amount_minor: i64, currency: impl Into<String>) -> Self {
        Self {
            amount_minor,
            currency: currency.into(),
        }
    }

    pub fn amount_minor(&self) -> i64 {
        self.amount_minor
    }

    pub fn currency(&self) -> &str {
        &self.currency
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn dt(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 10, hour, 0, 0).unwrap()
    }

    #[test]
    fn should_reject_a_range_whose_start_is_not_earlier_than_end() {
        let err = MeetingTimeRange::new(dt(11), dt(10)).unwrap_err();
        assert_eq!(err, DomainError::InvalidTimeRange);
        let err = MeetingTimeRange::new(dt(10), dt(10)).unwrap_err();
        assert_eq!(err, DomainError::InvalidTimeRange);
    }

    #[test]
    fn should_treat_touching_ranges_as_non_overlapping() {
        let first = MeetingTimeRange::new(dt(10), dt(11)).unwrap();
        let second = MeetingTimeRange::new(dt(11), dt(12)).unwrap();
        assert!(!first.overlaps(&second));
        assert!(!second.overlaps(&first));
    }

    #[test]
    fn should_detect_overlapping_ranges() {
        let first = MeetingTimeRange::new(dt(10), dt(12)).unwrap();
        let second = MeetingTimeRange::new(dt(11), dt(13)).unwrap();
        assert!(first.overlaps(&second));
    }

    #[test]
    fn should_construct_a_cancellation_fee_value_object() {
        let fee = CancellationFee::new(1500, "JPY");
        assert_eq!(fee.amount_minor(), 1500);
        assert_eq!(fee.currency(), "JPY");
    }
}
