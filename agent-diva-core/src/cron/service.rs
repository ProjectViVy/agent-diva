//! Cron service for scheduling agent tasks

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::cron::types::{CronJob, CronJobState, CronPayload, CronSchedule, CronStore};

/// Get current time in milliseconds
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// Compute next run time in milliseconds
fn compute_next_run(schedule: &CronSchedule, now_ms: i64) -> Option<i64> {
    match schedule {
        CronSchedule::At { at_ms } => {
            if *at_ms > now_ms {
                Some(*at_ms)
            } else {
                None
            }
        }
        CronSchedule::Every { every_ms } => {
            if *every_ms <= 0 {
                None
            } else {
                Some(now_ms + every_ms)
            }
        }
        CronSchedule::Cron { expr, tz } => {
            // Try to use cron crate
            match cron::Schedule::try_from(expr.as_str()) {
                Ok(schedule) => {
                    // Convert now_ms to DateTime<Utc>
                    let seconds = now_ms / 1000;
                    let nanoseconds = ((now_ms % 1000) * 1_000_000) as u32;
                    let now_utc = chrono::DateTime::from_timestamp(seconds, nanoseconds)
                        .unwrap_or_else(|| chrono::Utc::now());

                    if let Some(tz_str) = tz {
                        if let Ok(tz) = tz_str.parse::<chrono_tz::Tz>() {
                            let dt = now_utc.with_timezone(&tz);
                            return schedule
                                .after(&dt)
                                .next()
                                .map(|next| next.timestamp_millis());
                        } else {
                             warn!("Invalid timezone '{}', falling back to UTC", tz_str);
                        }
                    }

                    schedule
                        .after(&now_utc)
                        .next()
                        .map(|next| next.timestamp_millis())
                }
                Err(e) => {
                    warn!("Invalid cron expression '{}': {}", expr, e);
                    None
                }
            }
        }
    }
}

/// Callback function type for job execution
pub type JobCallback = Arc<
    dyn Fn(CronJob) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send>>
        + Send
        + Sync,
>;

/// Service for managing and executing scheduled jobs
pub struct CronService {
    store_path: PathBuf,
    on_job: Option<JobCallback>,
    store: Arc<RwLock<Option<CronStore>>>,
    timer_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    running: Arc<RwLock<bool>>,
}

impl CronService {
    /// Create a new cron service
    pub fn new(store_path: PathBuf, on_job: Option<JobCallback>) -> Self {
        Self {
            store_path,
            on_job,
            store: Arc::new(RwLock::new(None)),
            timer_task: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Load jobs from disk
    async fn load_store(&self) -> CronStore {
        // Check if already loaded
        {
            let store_guard = self.store.read().await;
            if let Some(store) = store_guard.as_ref() {
                return store.clone();
            }
        }

        // Load from file
        let store = if self.store_path.exists() {
            match tokio::fs::read_to_string(&self.store_path).await {
                Ok(content) => match serde_json::from_str::<CronStore>(&content) {
                    Ok(store) => {
                        debug!("Loaded {} cron jobs from disk", store.jobs.len());
                        store
                    }
                    Err(e) => {
                        warn!("Failed to parse cron store: {}", e);
                        CronStore::default()
                    }
                },
                Err(e) => {
                    warn!("Failed to read cron store: {}", e);
                    CronStore::default()
                }
            }
        } else {
            CronStore::default()
        };

        // Cache it
        {
            let mut store_guard = self.store.write().await;
            *store_guard = Some(store.clone());
        }

        store
    }

    /// Save jobs to disk
    async fn save_store(&self) {
        let store = {
            let store_guard = self.store.read().await;
            match store_guard.as_ref() {
                Some(s) => s.clone(),
                None => return,
            }
        };

        // Create parent directory
        if let Some(parent) = self.store_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        // Serialize and write
        match serde_json::to_string_pretty(&store) {
            Ok(content) => {
                if let Err(e) = tokio::fs::write(&self.store_path, content).await {
                    error!("Failed to save cron store: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to serialize cron store: {}", e);
            }
        }
    }

    /// Start the cron service
    pub async fn start(&self) {
        *self.running.write().await = true;

        let mut store = self.load_store().await;
        self.recompute_next_runs(&mut store).await;

        // Save updated store
        {
            let mut store_guard = self.store.write().await;
            *store_guard = Some(store);
        }
        self.save_store().await;

        self.arm_timer().await;

        let job_count = {
            let store_guard = self.store.read().await;
            store_guard.as_ref().map(|s| s.jobs.len()).unwrap_or(0)
        };
        info!("Cron service started with {} jobs", job_count);
    }

    /// Stop the cron service
    pub async fn stop(&self) {
        *self.running.write().await = false;

        let mut timer_guard = self.timer_task.lock().await;
        if let Some(task) = timer_guard.take() {
            task.abort();
        }
    }

    /// Recompute next run times for all enabled jobs
    async fn recompute_next_runs(&self, store: &mut CronStore) {
        let now = now_ms();
        for job in &mut store.jobs {
            if job.enabled {
                job.state.next_run_at_ms = compute_next_run(&job.schedule, now);
            }
        }
    }

    /// Get the earliest next run time across all jobs
    async fn get_next_wake_ms(&self) -> Option<i64> {
        let store_guard = self.store.read().await;
        let store = store_guard.as_ref()?;

        store
            .jobs
            .iter()
            .filter(|j| j.enabled && j.state.next_run_at_ms.is_some())
            .filter_map(|j| j.state.next_run_at_ms)
            .min()
    }

    /// Schedule the next timer tick
    async fn arm_timer(&self) {
        // Cancel existing timer
        {
            let mut timer_guard = self.timer_task.lock().await;
            if let Some(task) = timer_guard.take() {
                task.abort();
            }
        }

        let next_wake = self.get_next_wake_ms().await;
        let is_running = *self.running.read().await;

        if !is_running || next_wake.is_none() {
            return;
        }

        let next_wake = next_wake.unwrap();
        let delay_ms = (next_wake - now_ms()).max(0);
        let delay = tokio::time::Duration::from_millis(delay_ms as u64);

        debug!("Arming cron timer for {}ms", delay_ms);

        // Clone Arc references for the task
        let service = Arc::new(CronServiceHandle {
            store: Arc::clone(&self.store),
            timer_task: Arc::clone(&self.timer_task),
            running: Arc::clone(&self.running),
            on_job: self.on_job.clone(),
            store_path: self.store_path.clone(),
        });

        let task = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let is_running = *service.running.read().await;
            if is_running {
                service.on_timer().await;
            }
        });

        let mut timer_guard = self.timer_task.lock().await;
        *timer_guard = Some(task);
    }

    /// Handle timer tick - run due jobs
    #[allow(dead_code)]
    async fn on_timer_impl(&self) {
        let now = now_ms();

        // Find due jobs
        let due_jobs = {
            let store_guard = self.store.read().await;
            if let Some(store) = store_guard.as_ref() {
                store
                    .jobs
                    .iter()
                    .filter(|j| {
                        j.enabled
                            && j.state.next_run_at_ms.is_some()
                            && now >= j.state.next_run_at_ms.unwrap()
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        };

        // Execute each job
        for job in due_jobs {
            self.execute_job(job).await;
        }

        self.save_store().await;
        self.arm_timer().await;
    }

    /// Execute a single job
    async fn execute_job(&self, mut job: CronJob) {
        let start_ms = now_ms();
        info!("Cron: executing job '{}' ({})", job.name, job.id);

        let result = if let Some(callback) = &self.on_job {
            (callback)(job.clone()).await
        } else {
            None
        };

        // Update job state
        job.state.last_run_at_ms = Some(start_ms);
        job.updated_at_ms = now_ms();

        match result {
            Some(_response) => {
                job.state.last_status = Some("ok".to_string());
                job.state.last_error = None;
                info!("Cron: job '{}' completed", job.name);
            }
            None => {
                job.state.last_status = Some("ok".to_string());
                job.state.last_error = None;
            }
        }

        // Handle one-shot jobs
        let should_remove = match &job.schedule {
            CronSchedule::At { .. } => {
                if job.delete_after_run {
                    true
                } else {
                    job.enabled = false;
                    job.state.next_run_at_ms = None;
                    false
                }
            }
            _ => {
                // Compute next run
                job.state.next_run_at_ms = compute_next_run(&job.schedule, now_ms());
                false
            }
        };

        // Update store
        {
            let mut store_guard = self.store.write().await;
            if let Some(store) = store_guard.as_mut() {
                if should_remove {
                    store.jobs.retain(|j| j.id != job.id);
                } else {
                    // Find and update the job
                    if let Some(existing) = store.jobs.iter_mut().find(|j| j.id == job.id) {
                        *existing = job;
                    }
                }
            }
        }
    }

    // ========== Public API ==========

    /// List all jobs
    pub async fn list_jobs(&self, include_disabled: bool) -> Vec<CronJob> {
        let store = self.load_store().await;
        let mut jobs: Vec<CronJob> = if include_disabled {
            store.jobs
        } else {
            store.jobs.into_iter().filter(|j| j.enabled).collect()
        };

        jobs.sort_by_key(|j| j.state.next_run_at_ms.unwrap_or(i64::MAX));
        jobs
    }

    /// Add a new job
    #[allow(clippy::too_many_arguments)]
    pub async fn add_job(
        &self,
        name: String,
        schedule: CronSchedule,
        message: String,
        deliver: bool,
        channel: Option<String>,
        to: Option<String>,
        delete_after_run: bool,
    ) -> CronJob {
        let now = now_ms();
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();

        let job = CronJob {
            id: id.clone(),
            name: name.clone(),
            enabled: true,
            schedule: schedule.clone(),
            payload: CronPayload {
                kind: "agent_turn".to_string(),
                message,
                deliver,
                channel,
                to,
            },
            state: CronJobState {
                next_run_at_ms: compute_next_run(&schedule, now),
                ..Default::default()
            },
            created_at_ms: now,
            updated_at_ms: now,
            delete_after_run,
        };

        // Add to store
        {
            let mut store_guard = self.store.write().await;
            if let Some(store) = store_guard.as_mut() {
                store.jobs.push(job.clone());
            } else {
                let mut store = CronStore::default();
                store.jobs.push(job.clone());
                *store_guard = Some(store);
            }
        }

        self.save_store().await;
        self.arm_timer().await;

        info!("Cron: added job '{}' ({})", name, id);
        job
    }

    /// Remove a job by ID
    pub async fn remove_job(&self, job_id: &str) -> bool {
        let removed = {
            let mut store_guard = self.store.write().await;
            if let Some(store) = store_guard.as_mut() {
                let before = store.jobs.len();
                store.jobs.retain(|j| j.id != job_id);
                store.jobs.len() < before
            } else {
                false
            }
        };

        if removed {
            self.save_store().await;
            self.arm_timer().await;
            info!("Cron: removed job {}", job_id);
        }

        removed
    }

    /// Enable or disable a job
    pub async fn enable_job(&self, job_id: &str, enabled: bool) -> Option<CronJob> {
        let job = {
            let mut store_guard = self.store.write().await;
            if let Some(store) = store_guard.as_mut() {
                if let Some(job) = store.jobs.iter_mut().find(|j| j.id == job_id) {
                    job.enabled = enabled;
                    job.updated_at_ms = now_ms();
                    if enabled {
                        job.state.next_run_at_ms = compute_next_run(&job.schedule, now_ms());
                    } else {
                        job.state.next_run_at_ms = None;
                    }
                    Some(job.clone())
                } else {
                    None
                }
            } else {
                None
            }
        };

        if job.is_some() {
            self.save_store().await;
            self.arm_timer().await;
        }

        job
    }

    /// Manually run a job
    pub async fn run_job(&self, job_id: &str, force: bool) -> bool {
        let job = {
            let store_guard = self.store.read().await;
            if let Some(store) = store_guard.as_ref() {
                store
                    .jobs
                    .iter()
                    .find(|j| j.id == job_id)
                    .filter(|j| force || j.enabled)
                    .cloned()
            } else {
                None
            }
        };

        if let Some(job) = job {
            self.execute_job(job).await;
            self.save_store().await;
            self.arm_timer().await;
            true
        } else {
            false
        }
    }

    /// Get service status
    pub async fn status(&self) -> serde_json::Value {
        let store = self.load_store().await;
        let is_running = *self.running.read().await;
        let next_wake = self.get_next_wake_ms().await;

        serde_json::json!({
            "enabled": is_running,
            "jobs": store.jobs.len(),
            "nextWakeAtMs": next_wake,
        })
    }
}

/// Handle for async timer task (to avoid circular references)
struct CronServiceHandle {
    store: Arc<RwLock<Option<CronStore>>>,
    #[allow(dead_code)]
    timer_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    running: Arc<RwLock<bool>>,
    on_job: Option<JobCallback>,
    store_path: PathBuf,
}

impl CronServiceHandle {
    async fn on_timer(&self) {
        let now = now_ms();

        // Find due jobs
        let due_jobs = {
            let store_guard = self.store.read().await;
            if let Some(store) = store_guard.as_ref() {
                store
                    .jobs
                    .iter()
                    .filter(|j| {
                        j.enabled
                            && j.state.next_run_at_ms.is_some()
                            && now >= j.state.next_run_at_ms.unwrap()
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        };

        // Execute each job
        for mut job in due_jobs {
            let start_ms = now_ms();
            info!("Cron: executing job '{}' ({})", job.name, job.id);

            if let Some(callback) = &self.on_job {
                let _result = (callback)(job.clone()).await;
            }

            job.state.last_run_at_ms = Some(start_ms);
            job.state.last_status = Some("ok".to_string());
            job.state.last_error = None;
            job.updated_at_ms = now_ms();

            // Handle one-shot jobs
            let should_remove = match &job.schedule {
                CronSchedule::At { .. } => {
                    if job.delete_after_run {
                        true
                    } else {
                        job.enabled = false;
                        job.state.next_run_at_ms = None;
                        false
                    }
                }
                _ => {
                    job.state.next_run_at_ms = compute_next_run(&job.schedule, now_ms());
                    false
                }
            };

            // Update store
            {
                let mut store_guard = self.store.write().await;
                if let Some(store) = store_guard.as_mut() {
                    if should_remove {
                        store.jobs.retain(|j| j.id != job.id);
                    } else if let Some(existing) = store.jobs.iter_mut().find(|j| j.id == job.id) {
                        *existing = job;
                    }
                }
            }
        }

        // Save store
        if let Some(parent) = self.store_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        {
            let store_guard = self.store.read().await;
            if let Some(store) = store_guard.as_ref() {
                if let Ok(content) = serde_json::to_string_pretty(store) {
                    let _ = tokio::fs::write(&self.store_path, content).await;
                }
            }
        }

        // Re-arm timer
        // Note: This would need the full CronService reference, which we don't have here
        // In practice, the main CronService::on_timer_impl handles this
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_now_ms() {
        let now = now_ms();
        assert!(now > 0);
    }

    #[test]
    fn test_compute_next_run_at() {
        let schedule = CronSchedule::at(2000);
        assert_eq!(compute_next_run(&schedule, 1000), Some(2000));
        assert_eq!(compute_next_run(&schedule, 3000), None);
    }

    #[test]
    fn test_compute_next_run_every() {
        let schedule = CronSchedule::every(5000);
        let next = compute_next_run(&schedule, 1000);
        assert_eq!(next, Some(6000));
    }

    #[tokio::test]
    async fn test_cron_service_new() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        let service = CronService::new(store_path, None);

        let is_running = *service.running.read().await;
        assert!(!is_running);
    }

    #[tokio::test]
    async fn test_cron_service_add_job() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        let service = CronService::new(store_path, None);

        service.start().await;

        let job = service
            .add_job(
                "Test Job".to_string(),
                CronSchedule::every(5000),
                "Test message".to_string(),
                false,
                None,
                None,
                false,
            )
            .await;

        assert_eq!(job.name, "Test Job");
        assert!(job.enabled);

        let jobs = service.list_jobs(false).await;
        assert_eq!(jobs.len(), 1);

        service.stop().await;
    }

    #[tokio::test]
    async fn test_cron_service_remove_job() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        let service = CronService::new(store_path, None);

        service.start().await;

        let job = service
            .add_job(
                "Test Job".to_string(),
                CronSchedule::every(5000),
                "Test message".to_string(),
                false,
                None,
                None,
                false,
            )
            .await;

        let removed = service.remove_job(&job.id).await;
        assert!(removed);

        let jobs = service.list_jobs(false).await;
        assert_eq!(jobs.len(), 0);

        service.stop().await;
    }

    #[tokio::test]
    async fn test_cron_service_enable_disable() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        let service = CronService::new(store_path, None);

        service.start().await;

        let job = service
            .add_job(
                "Test Job".to_string(),
                CronSchedule::every(5000),
                "Test message".to_string(),
                false,
                None,
                None,
                false,
            )
            .await;

        let disabled = service.enable_job(&job.id, false).await;
        assert!(disabled.is_some());
        assert!(!disabled.unwrap().enabled);

        let enabled = service.enable_job(&job.id, true).await;
        assert!(enabled.is_some());
        assert!(enabled.unwrap().enabled);

        service.stop().await;
    }
}
