use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Report period covered by a rhythm / notebook report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportPeriod {
    Daily,
    Weekly,
    Monthly,
}

impl ReportPeriod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }
}

/// Inclusive calendar window for a report generation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportWindow {
    pub period: ReportPeriod,
    /// Stable key: `YYYY-MM-DD`, `YYYY-Www`, or `YYYY-MM`.
    pub key: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl ReportWindow {
    pub fn daily(date: NaiveDate) -> Self {
        Self {
            period: ReportPeriod::Daily,
            key: date.format("%Y-%m-%d").to_string(),
            start: date,
            end: date,
        }
    }

    pub fn weekly(year: i32, week: u32, start: NaiveDate, end: NaiveDate) -> Self {
        Self {
            period: ReportPeriod::Weekly,
            key: format!("{year}-W{week:02}"),
            start,
            end,
        }
    }

    pub fn monthly(year: i32, month: u32, start: NaiveDate, end: NaiveDate) -> Self {
        Self {
            period: ReportPeriod::Monthly,
            key: format!("{year:04}-{month:02}"),
            start,
            end,
        }
    }
}
