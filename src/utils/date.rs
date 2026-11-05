use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub const DATE_FORMAT: &str = "%Y-%m-%d";

pub fn today_local() -> NaiveDate {
    chrono::Local::now().date_naive()
}

pub fn now_timestamp() -> String {
    chrono::Local::now().to_rfc3339()
}

pub fn format_date(date: NaiveDate) -> String {
    date.format(DATE_FORMAT).to_string()
}

pub fn parse_date(value: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), DATE_FORMAT)
        .map_err(|_| AppError::validation(format!("'{value}' is not a valid date (YYYY-MM-DD)")))
}

/// Parses a user supplied date or reports which field was invalid.
pub fn parse_date_field(value: &str, field: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), DATE_FORMAT)
        .map_err(|_| AppError::validation(format!("{field} must be a valid date (YYYY-MM-DD)")))
}

pub fn parse_optional_date_field(value: &str, field: &str) -> AppResult<Option<NaiveDate>> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_date_field(value, field).map(Some)
    }
}

/// Inclusive reporting window. `None` means unbounded (all time).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
}

impl DateRange {
    pub fn new(start: Option<NaiveDate>, end: Option<NaiveDate>) -> AppResult<Self> {
        if let (Some(start), Some(end)) = (start, end) {
            if start > end {
                return Err(AppError::validation(
                    "The start date must be on or before the end date",
                ));
            }
        }
        Ok(Self { start, end })
    }

    pub fn all_time() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    pub fn contains(&self, date: NaiveDate) -> bool {
        self.start.is_none_or(|start| date >= start) && self.end.is_none_or(|end| date <= end)
    }
}

/// Reporting periods exposed in the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Period {
    Last7Days,
    Last30Days,
    Last90Days,
    Last6Months,
    Last12Months,
    AllTime,
    Custom(DateRange),
}

impl Period {
    pub fn resolve(&self, today: NaiveDate) -> AppResult<DateRange> {
        let inclusive_days =
            |days: i64| DateRange::new(Some(today - Duration::days(days - 1)), Some(today));
        let inclusive_months = |months: u32| {
            let start = add_months(today, -(months as i32));
            DateRange::new(Some(start), Some(today))
        };
        match self {
            Self::Last7Days => inclusive_days(7),
            Self::Last30Days => inclusive_days(30),
            Self::Last90Days => inclusive_days(90),
            Self::Last6Months => inclusive_months(6),
            Self::Last12Months => inclusive_months(12),
            Self::AllTime => Ok(DateRange::all_time()),
            Self::Custom(range) => DateRange::new(range.start, range.end),
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Last7Days => "7",
            Self::Last30Days => "30",
            Self::Last90Days => "90",
            Self::Last6Months => "6m",
            Self::Last12Months => "12m",
            Self::AllTime => "all",
            Self::Custom(_) => "custom",
        }
    }

    pub fn from_key(key: &str) -> AppResult<Self> {
        match key {
            "7" => Ok(Self::Last7Days),
            "30" => Ok(Self::Last30Days),
            "90" => Ok(Self::Last90Days),
            "6m" => Ok(Self::Last6Months),
            "12m" => Ok(Self::Last12Months),
            "all" => Ok(Self::AllTime),
            "custom" => Err(AppError::validation(
                "Choose both custom dates and apply the range",
            )),
            other => Err(AppError::validation(format!(
                "Unknown reporting period '{other}'"
            ))),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Last7Days => "Last 7 days".into(),
            Self::Last30Days => "Last 30 days".into(),
            Self::Last90Days => "Last 90 days".into(),
            Self::Last6Months => "Last 6 months".into(),
            Self::Last12Months => "Last 12 months".into(),
            Self::AllTime => "All time".into(),
            Self::Custom(range) => match (range.start, range.end) {
                (Some(start), Some(end)) => {
                    format!("{} to {}", format_date(start), format_date(end))
                }
                _ => "Custom range".into(),
            },
        }
    }
}

/// Adds months while clamping the day to the last valid day of the target month.
pub fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    let total = date.year() * 12 + date.month0() as i32 + months;
    let year = total.div_euclid(12);
    let month0 = total.rem_euclid(12) as u32;
    let month = month0 + 1;
    let last_day = last_day_of_month(year, month);
    NaiveDate::from_ymd_opt(year, month, date.day().min(last_day)).unwrap_or(date)
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    (1..=31)
        .rev()
        .find(|day| NaiveDate::from_ymd_opt(year, month, *day).is_some())
        .unwrap_or(28)
}

/// Monday of the ISO week containing `date`.
pub fn week_start(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

/// Human-friendly short date label used by lists and chart captions.
pub fn display_date(date: NaiveDate) -> String {
    date.format("%d %b %Y").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn period_windows_are_inclusive() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let range = Period::Last7Days.resolve(today).unwrap();
        assert_eq!(
            range.start,
            Some(NaiveDate::from_ymd_opt(2026, 10, 30).unwrap())
        );
        assert_eq!(range.end, Some(today));
    }

    #[test]
    fn month_math_clamps_leap_days() {
        let leap = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        assert_eq!(
            add_months(leap, -12),
            NaiveDate::from_ymd_opt(2023, 2, 28).unwrap()
        );
    }

    #[test]
    fn custom_range_rejects_reversed_dates() {
        let start = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
        assert!(DateRange::new(Some(start), Some(end)).is_err());
    }

    #[test]
    fn week_starts_on_monday() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        assert_eq!(
            week_start(date),
            NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()
        );
    }
}
