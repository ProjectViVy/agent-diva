//! Token usage query service for statistics and analytics

use crate::usage::types::{OperationType, SessionUsage, TimelinePoint, UsageSummary, UsageTotal};
use crate::Result;
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::debug;

/// Time range for queries
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    /// Last 1 day
    #[default]
    Last1Day,
    /// Last 3 days
    Last3Days,
    /// Last 1 week
    Last1Week,
    /// Last 1 month
    Last1Month,
    /// Last 6 months
    Last6Months,
    /// Last 1 year
    Last1Year,
    /// Custom time range
    Custom {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
}

impl TimeRange {
    /// Convert to (start, end) timestamp tuple
    pub fn to_bounds(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let now = Utc::now();
        match self {
            TimeRange::Last1Day => (now - Duration::days(1), now),
            TimeRange::Last3Days => (now - Duration::days(3), now),
            TimeRange::Last1Week => (now - Duration::weeks(1), now),
            TimeRange::Last1Month => (now - Duration::days(30), now),
            TimeRange::Last6Months => (now - Duration::days(180), now),
            TimeRange::Last1Year => (now - Duration::days(365), now),
            TimeRange::Custom { start, end } => (*start, *end),
        }
    }

    /// Parse from period string (e.g., "1d", "3d", "1w", "1m", "6m", "1y")
    pub fn parse_period(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "1d" => Some(Self::Last1Day),
            "3d" => Some(Self::Last3Days),
            "1w" => Some(Self::Last1Week),
            "1m" => Some(Self::Last1Month),
            "6m" => Some(Self::Last6Months),
            "1y" => Some(Self::Last1Year),
            _ => None,
        }
    }

    /// Get interval for timeline queries
    pub fn timeline_interval(&self) -> TimeInterval {
        match self {
            TimeRange::Last1Day => TimeInterval::Hour,
            TimeRange::Last3Days => TimeInterval::Hour,
            _ => TimeInterval::Day,
        }
    }
}

/// Time interval for timeline bucketing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeInterval {
    /// Hourly buckets
    Hour,
    /// Daily buckets
    Day,
}

/// Grouping dimension for summary queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupBy {
    /// Group by endpoint/provider
    #[default]
    Endpoint,
    /// Group by model
    Model,
    /// Group by operation type
    OperationType,
    /// Group by session
    Session,
    /// Group by channel
    Channel,
    /// Group by agent profile
    AgentProfile,
}

impl GroupBy {
    /// Convert to SQL column name
    pub fn to_column_name(&self) -> &'static str {
        match self {
            GroupBy::Endpoint => "endpoint_name",
            GroupBy::Model => "model",
            GroupBy::OperationType => "operation_type",
            GroupBy::Session => "session_id",
            GroupBy::Channel => "channel",
            GroupBy::AgentProfile => "agent_profile_id",
        }
    }

    /// Parse from dimension string
    pub fn parse_dimension(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "endpoint" | "endpoint_name" => Some(Self::Endpoint),
            "model" => Some(Self::Model),
            "operation" | "operation_type" => Some(Self::OperationType),
            "session" | "session_id" => Some(Self::Session),
            "channel" => Some(Self::Channel),
            "agent" | "agent_profile" | "agent_profile_id" => Some(Self::AgentProfile),
            _ => None,
        }
    }
}

/// Usage query service with thread-safe database access
pub struct UsageQueryService {
    /// Database connection protected by Mutex for thread safety
    db: Mutex<Connection>,
}

impl UsageQueryService {
    /// Create a new query service from database path
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(&db_path)?;
        Ok(Self {
            db: Mutex::new(conn),
        })
    }

    /// Create from existing connection
    pub fn from_connection(conn: Connection) -> Self {
        Self {
            db: Mutex::new(conn),
        }
    }

    /// Get total usage statistics
    pub fn get_total(&self, range: TimeRange) -> Result<UsageTotal> {
        let (start, end) = range.to_bounds();
        let db = self
            .db
            .lock()
            .map_err(|e| crate::Error::Database(e.to_string()))?;

        let sql = "SELECT
            COALESCE(SUM(input_tokens), 0) as total_input,
            COALESCE(SUM(output_tokens), 0) as total_output,
            COALESCE(SUM(input_tokens + output_tokens), 0) as total_tokens,
            COALESCE(SUM(cache_creation_tokens), 0) as total_cache_creation,
            COALESCE(SUM(cache_read_tokens), 0) as total_cache_read,
            COUNT(*) as request_count,
            COALESCE(SUM(estimated_cost), 0) as total_cost
        FROM token_usage
        WHERE timestamp >= ?1 AND timestamp <= ?2";

        let total = db.query_row(sql, params![start.to_rfc3339(), end.to_rfc3339()], |row| {
            Ok(UsageTotal {
                total_input: row.get(0)?,
                total_output: row.get(1)?,
                total_tokens: row.get(2)?,
                total_cache_creation: row.get(3)?,
                total_cache_read: row.get(4)?,
                request_count: row.get::<_, i64>(5)? as u64,
                total_cost: row.get(6)?,
            })
        })?;

        debug!("Total usage query: {:?}", total);
        Ok(total)
    }

    /// Get grouped usage summary
    pub fn get_summary(&self, range: TimeRange, group_by: GroupBy) -> Result<Vec<UsageSummary>> {
        let (start, end) = range.to_bounds();
        let group_column = group_by.to_column_name();
        let db = self
            .db
            .lock()
            .map_err(|e| crate::Error::Database(e.to_string()))?;

        let sql = format!(
            "SELECT
                {group_column} as group_key,
                COALESCE(SUM(input_tokens), 0) as total_input,
                COALESCE(SUM(output_tokens), 0) as total_output,
                COALESCE(SUM(input_tokens + output_tokens), 0) as total_tokens,
                COALESCE(SUM(cache_creation_tokens), 0) as total_cache_creation,
                COALESCE(SUM(cache_read_tokens), 0) as total_cache_read,
                COUNT(*) as request_count,
                COALESCE(SUM(estimated_cost), 0) as total_cost
            FROM token_usage
            WHERE timestamp >= ?1 AND timestamp <= ?2
            GROUP BY {group_column}
            ORDER BY total_tokens DESC
            LIMIT 20"
        );

        let mut stmt = db.prepare(&sql)?;
        let summaries = stmt
            .query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
                Ok(UsageSummary {
                    group_key: row.get(0)?,
                    total_input: row.get(1)?,
                    total_output: row.get(2)?,
                    total_tokens: row.get(3)?,
                    total_cache_creation: row.get(4)?,
                    total_cache_read: row.get(5)?,
                    request_count: row.get::<_, i64>(6)? as u64,
                    total_cost: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        debug!(
            "Summary query by {}: {} results",
            group_column,
            summaries.len()
        );
        Ok(summaries)
    }

    /// Get timeline data for charting
    pub fn get_timeline(
        &self,
        range: TimeRange,
        interval: TimeInterval,
    ) -> Result<Vec<TimelinePoint>> {
        let (start, end) = range.to_bounds();

        // SQLite strftime format for time bucketing
        let time_format = match interval {
            TimeInterval::Hour => "%Y-%m-%d %H:00",
            TimeInterval::Day => "%Y-%m-%d",
        };

        let db = self
            .db
            .lock()
            .map_err(|e| crate::Error::Database(e.to_string()))?;

        let sql = format!(
            "SELECT
                strftime('{}', timestamp) as time_bucket,
                COALESCE(SUM(input_tokens), 0) as total_input,
                COALESCE(SUM(output_tokens), 0) as total_output,
                COALESCE(SUM(input_tokens + output_tokens), 0) as total_tokens,
                COUNT(*) as request_count
            FROM token_usage
            WHERE timestamp >= ?1 AND timestamp <= ?2
            GROUP BY time_bucket
            ORDER BY time_bucket ASC",
            time_format
        );

        let mut stmt = db.prepare(&sql)?;
        let timeline = stmt
            .query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
                Ok(TimelinePoint {
                    time_bucket: row.get(0)?,
                    total_input: row.get(1)?,
                    total_output: row.get(2)?,
                    total_tokens: row.get(3)?,
                    request_count: row.get::<_, i64>(4)? as u64,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        debug!("Timeline query: {} points", timeline.len());
        Ok(timeline)
    }

    /// Get session usage details
    pub fn get_sessions(&self, range: TimeRange, limit: u64) -> Result<Vec<SessionUsage>> {
        let (start, end) = range.to_bounds();
        let db = self
            .db
            .lock()
            .map_err(|e| crate::Error::Database(e.to_string()))?;

        let sql = "SELECT
            session_id,
            COALESCE(SUM(input_tokens), 0) as total_input,
            COALESCE(SUM(output_tokens), 0) as total_output,
            COALESCE(SUM(input_tokens + output_tokens), 0) as total_tokens,
            COUNT(*) as request_count,
            COALESCE(SUM(estimated_cost), 0) as total_cost,
            MAX(timestamp) as last_activity
        FROM token_usage
        WHERE timestamp >= ?1 AND timestamp <= ?2
        GROUP BY session_id
        ORDER BY total_tokens DESC
        LIMIT ?3";

        let mut stmt = db.prepare(sql)?;
        let sessions = stmt
            .query_map(
                params![start.to_rfc3339(), end.to_rfc3339(), limit as i64],
                |row| {
                    let session_id: String = row.get(0)?;
                    let last_activity_str: String = row.get(7)?;
                    let last_activity = DateTime::parse_from_rfc3339(&last_activity_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());

                    // Get primary model for this session
                    let model_sql = "SELECT model, COUNT(*) as cnt FROM token_usage WHERE session_id = ?1 GROUP BY model ORDER BY cnt DESC LIMIT 1";
                    let primary_model = db.query_row(model_sql, params![&session_id], |r| r.get::<_, String>(0))
                        .unwrap_or_else(|_| "unknown".to_string());

                    // Get channel for this session
                    let channel_sql = "SELECT channel FROM token_usage WHERE session_id = ?1 AND channel IS NOT NULL LIMIT 1";
                    let channel = db.query_row(channel_sql, params![&session_id], |r| r.get::<_, Option<String>>(0))
                        .unwrap_or(None);

                    Ok(SessionUsage {
                        session_id,
                        total_input: row.get(1)?,
                        total_output: row.get(2)?,
                        total_tokens: row.get(3)?,
                        request_count: row.get::<_, i64>(4)? as u64,
                        total_cost: row.get(5)?,
                        primary_model,
                        channel,
                        last_activity,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        debug!("Session usage query: {} sessions", sessions.len());
        Ok(sessions)
    }

    /// Get model distribution percentages
    pub fn get_model_distribution(&self, range: TimeRange) -> Result<Vec<(String, f64)>> {
        let (start, end) = range.to_bounds();
        let db = self
            .db
            .lock()
            .map_err(|e| crate::Error::Database(e.to_string()))?;

        let sql = "SELECT
            model,
            SUM(input_tokens + output_tokens) as total
        FROM token_usage
        WHERE timestamp >= ?1 AND timestamp <= ?2
        GROUP BY model
        ORDER BY total DESC";

        let mut stmt = db.prepare(sql)?;
        let model_totals: Vec<(String, i64)> = stmt
            .query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let grand_total = model_totals.iter().map(|(_, t)| *t).sum::<i64>();
        let distribution = if grand_total > 0 {
            model_totals
                .iter()
                .map(|(model, total)| (model.clone(), (*total as f64 / grand_total as f64) * 100.0))
                .collect()
        } else {
            Vec::new()
        };

        Ok(distribution)
    }

    /// Get usage by operation type
    pub fn get_by_operation(&self, range: TimeRange) -> Result<Vec<(OperationType, UsageTotal)>> {
        let (start, end) = range.to_bounds();
        let db = self
            .db
            .lock()
            .map_err(|e| crate::Error::Database(e.to_string()))?;

        let sql = "SELECT
            operation_type,
            COALESCE(SUM(input_tokens), 0) as total_input,
            COALESCE(SUM(output_tokens), 0) as total_output,
            COALESCE(SUM(input_tokens + output_tokens), 0) as total_tokens,
            COALESCE(SUM(cache_creation_tokens), 0) as total_cache_creation,
            COALESCE(SUM(cache_read_tokens), 0) as total_cache_read,
            COUNT(*) as request_count,
            COALESCE(SUM(estimated_cost), 0) as total_cost
        FROM token_usage
        WHERE timestamp >= ?1 AND timestamp <= ?2
        GROUP BY operation_type
        ORDER BY total_tokens DESC";

        let mut stmt = db.prepare(sql)?;
        let results = stmt
            .query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
                let op_str: String = row.get(0)?;
                let op_type = op_str
                    .parse::<OperationType>()
                    .unwrap_or(OperationType::Unknown);
                let total = UsageTotal {
                    total_input: row.get(1)?,
                    total_output: row.get(2)?,
                    total_tokens: row.get(3)?,
                    total_cache_creation: row.get(4)?,
                    total_cache_read: row.get(5)?,
                    request_count: row.get::<_, i64>(6)? as u64,
                    total_cost: row.get(7)?,
                };
                Ok((op_type, total))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::types::TokenUsageRecord;
    use crate::usage::writer::TokenUsageWriter;
    use tempfile::TempDir;

    #[test]
    fn test_time_range_bounds() {
        let range = TimeRange::Last1Day;
        let (start, end) = range.to_bounds();
        assert!(end > start);
        let diff = end - start;
        assert!(diff.num_hours() <= 24);
    }

    #[test]
    fn test_time_range_parse() {
        assert_eq!(TimeRange::parse_period("1d"), Some(TimeRange::Last1Day));
        assert_eq!(TimeRange::parse_period("1w"), Some(TimeRange::Last1Week));
        assert_eq!(TimeRange::parse_period("invalid"), None);
    }

    #[test]
    fn test_group_by_column() {
        assert_eq!(GroupBy::Endpoint.to_column_name(), "endpoint_name");
        assert_eq!(GroupBy::Model.to_column_name(), "model");
    }

    #[tokio::test]
    async fn test_query_service() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("token_usage.db");

        // Start writer and insert some records
        let writer = TokenUsageWriter::start(db_path.clone());

        for i in 0..5 {
            let record = TokenUsageRecord::new(
                format!("test:session-{}", i),
                "openrouter",
                "claude-3-5-sonnet",
                OperationType::Chat,
                1000 * (i + 1),
                500 * (i + 1),
            );
            writer.record(record);
        }

        // Wait for flush
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        writer.stop();

        // Query service
        let query = UsageQueryService::new(db_path).unwrap();
        let total = query.get_total(TimeRange::Last1Day).unwrap();

        assert!(total.request_count >= 5);
        assert!(total.total_tokens >= 7500);
    }
}
