//! Cron service for scheduling agent tasks

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::cron::types::{
    CreateCronJobRequest, CronJob, CronJobDto, CronJobLifecycleStatus, CronJobState, CronPayload,
    CronRunSnapshot, CronSchedule, CronStore, CronTrigger, UpdateCronJobRequest,
};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn normalize_cron_expr(expr: &str) -> String {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() == 5 {
        format!("0 {}", fields.join(" "))
    } else {
        fields.join(" ")
    }
}

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
            match cron::Schedule::try_from(normalize_cron_expr(expr).as_str()) {
                Ok(schedule) => {
                    let seconds = now_ms / 1000;
                    let nanoseconds = ((now_ms % 1000) * 1_000_000) as u32;
                    let now_utc = chrono::DateTime::from_timestamp(seconds, nanoseconds)
                        .unwrap_or_else(chrono::Utc::now);

                    if let Some(tz_str) = tz {
                        if let Ok(tz) = tz_str.parse::<chrono_tz::Tz>() {
                            let dt = now_utc.with_timezone(&tz);
                            return schedule
                                .after(&dt)
                                .next()
                                .map(|next| next.timestamp_millis());
                        }
                        warn!("Invalid timezone '{}', falling back to UTC", tz_str);
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

pub type JobCallback = Arc<
    dyn Fn(
            CronJob,
            CancellationToken,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send>>
        + Send
        + Sync,
>;

#[derive(Clone)]
struct ActiveCronRun {
    snapshot: CronRunSnapshot,
    cancel_token: CancellationToken,
}

pub struct CronService {
    store_path: PathBuf,
    on_job: Option<JobCallback>,
    store: Arc<RwLock<Option<CronStore>>>,
    timer_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    running: Arc<RwLock<bool>>,
    active_runs: Arc<RwLock<HashMap<String, ActiveCronRun>>>,
}

impl CronService {
    pub fn new(store_path: PathBuf, on_job: Option<JobCallback>) -> Self {
        Self {
            store_path,
            on_job,
            store: Arc::new(RwLock::new(None)),
            timer_task: Arc::new(Mutex::new(None)),
            running: Arc::new(RwLock::new(false)),
            active_runs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn load_store(&self) -> CronStore {
        {
            let store_guard = self.store.read().await;
            if let Some(store) = store_guard.as_ref() {
                return store.clone();
            }
        }

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

        let mut store_guard = self.store.write().await;
        *store_guard = Some(store.clone());
        store
    }

    async fn save_store(&self) {
        let store = {
            let store_guard = self.store.read().await;
            match store_guard.as_ref() {
                Some(s) => s.clone(),
                None => return,
            }
        };

        if let Some(parent) = self.store_path.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }

        match serde_json::to_string_pretty(&store) {
            Ok(content) => {
                if let Err(e) = tokio::fs::write(&self.store_path, content).await {
                    error!("Failed to save cron store: {}", e);
                }
            }
            Err(e) => error!("Failed to serialize cron store: {}", e),
        }
    }

    pub async fn start(&self) {
        *self.running.write().await = true;

        let mut store = self.load_store().await;
        self.recompute_next_runs(&mut store).await;

        let mut store_guard = self.store.write().await;
        *store_guard = Some(store);
        drop(store_guard);

        self.save_store().await;
        self.arm_timer().await;
    }

    pub async fn stop(&self) {
        *self.running.write().await = false;

        {
            let mut timer_guard = self.timer_task.lock().await;
            if let Some(task) = timer_guard.take() {
                task.abort();
            }
        }

        let tokens = {
            let active_guard = self.active_runs.read().await;
            active_guard
                .values()
                .map(|run| run.cancel_token.clone())
                .collect::<Vec<_>>()
        };
        for token in tokens {
            token.cancel();
        }
    }

    async fn recompute_next_runs(&self, store: &mut CronStore) {
        let now = now_ms();
        for job in &mut store.jobs {
            if job.enabled {
                job.state.next_run_at_ms = compute_next_run(&job.schedule, now);
            } else {
                job.state.next_run_at_ms = None;
            }
        }
    }

    async fn get_next_wake_ms(&self) -> Option<i64> {
        let store_guard = self.store.read().await;
        let store = store_guard.as_ref()?;

        store
            .jobs
            .iter()
            .filter(|job| job.enabled && job.state.next_run_at_ms.is_some())
            .filter_map(|job| job.state.next_run_at_ms)
            .min()
    }

    async fn arm_timer(&self) {
        {
            let mut timer_guard = self.timer_task.lock().await;
            if let Some(task) = timer_guard.take() {
                task.abort();
            }
        }

        let next_wake = self.get_next_wake_ms().await;
        let is_running = *self.running.read().await;
        if !is_running {
            return;
        }

        let Some(next_wake) = next_wake else {
            return;
        };

        let delay_ms = (next_wake - now_ms()).max(0);
        let delay = tokio::time::Duration::from_millis(delay_ms as u64);
        let service = Arc::new(CronServiceHandle {
            store: Arc::clone(&self.store),
            timer_task: Arc::clone(&self.timer_task),
            running: Arc::clone(&self.running),
            on_job: self.on_job.clone(),
            store_path: self.store_path.clone(),
            active_runs: Arc::clone(&self.active_runs),
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

    fn to_job_dto(job: &CronJob, active_run: Option<CronRunSnapshot>) -> CronJobDto {
        let computed_status = if active_run.is_some() {
            CronJobLifecycleStatus::Running
        } else if !job.enabled {
            CronJobLifecycleStatus::Paused
        } else if job.state.last_status.as_deref() == Some("error") {
            CronJobLifecycleStatus::Failed
        } else if job.state.last_run_at_ms.is_some() {
            CronJobLifecycleStatus::Completed
        } else {
            CronJobLifecycleStatus::Scheduled
        };

        CronJobDto {
            job: job.clone(),
            is_running: active_run.is_some(),
            active_run,
            computed_status,
        }
    }

    async fn active_snapshot_for(&self, job_id: &str) -> Option<CronRunSnapshot> {
        let active_guard = self.active_runs.read().await;
        active_guard.get(job_id).map(|run| run.snapshot.clone())
    }

    async fn register_active_run(
        &self,
        job_id: &str,
        trigger: CronTrigger,
    ) -> Result<CronRunSnapshot, String> {
        let mut active_guard = self.active_runs.write().await;
        if active_guard.contains_key(job_id) {
            return Err(format!("job {} is already running", job_id));
        }

        let timestamp = now_ms();
        let snapshot = CronRunSnapshot {
            run_id: uuid::Uuid::new_v4().to_string(),
            job_id: job_id.to_string(),
            started_at_ms: timestamp,
            last_heartbeat_at_ms: timestamp,
            trigger,
            cancelable: true,
        };

        active_guard.insert(
            job_id.to_string(),
            ActiveCronRun {
                snapshot: snapshot.clone(),
                cancel_token: CancellationToken::new(),
            },
        );
        Ok(snapshot)
    }

    async fn cancel_token_for(&self, job_id: &str) -> Option<CancellationToken> {
        let active_guard = self.active_runs.read().await;
        active_guard.get(job_id).map(|run| run.cancel_token.clone())
    }

    async fn clear_active_run(&self, job_id: &str) {
        let mut active_guard = self.active_runs.write().await;
        active_guard.remove(job_id);
    }

    async fn execute_job_with_trigger(
        &self,
        mut job: CronJob,
        trigger: CronTrigger,
    ) -> Result<CronJobDto, String> {
        info!(
            "Cron: executing job '{}' ({}) trigger={:?}",
            job.name, job.id, trigger
        );
        let _ = self.register_active_run(&job.id, trigger).await?;
        let cancel_token = self
            .cancel_token_for(&job.id)
            .await
            .ok_or_else(|| format!("missing cancel token for {}", job.id))?;

        let callback_result = if cancel_token.is_cancelled() {
            Err("job cancelled before start".to_string())
        } else if let Some(callback) = &self.on_job {
            match (callback)(job.clone(), cancel_token.clone()).await {
                Some(response) if cancel_token.is_cancelled() => Err("job cancelled".to_string()),
                Some(response) if response.to_ascii_lowercase().starts_with("error") => {
                    Err(response)
                }
                Some(_) | None => Ok(()),
            }
        } else if cancel_token.is_cancelled() {
            Err("job cancelled".to_string())
        } else {
            Ok(())
        };

        let now = now_ms();
        job.state.last_run_at_ms = Some(now);
        job.updated_at_ms = now;
        match callback_result {
            Ok(()) => {
                job.state.last_status = Some("ok".to_string());
                job.state.last_error = None;
                info!(
                    "Cron: job '{}' ({}) completed successfully",
                    job.name, job.id
                );
            }
            Err(err) => {
                job.state.last_status = Some("error".to_string());
                job.state.last_error = Some(err);
                warn!("Cron: job '{}' ({}) failed", job.name, job.id);
            }
        }

        let should_remove = match &job.schedule {
            CronSchedule::At { .. } => {
                if job.delete_after_run && job.state.last_status.as_deref() == Some("ok") {
                    true
                } else {
                    job.enabled = false;
                    job.state.next_run_at_ms = None;
                    false
                }
            }
            _ => {
                job.state.next_run_at_ms = if job.enabled {
                    compute_next_run(&job.schedule, now_ms())
                } else {
                    None
                };
                false
            }
        };

        {
            let mut store_guard = self.store.write().await;
            if let Some(store) = store_guard.as_mut() {
                if should_remove {
                    store.jobs.retain(|existing| existing.id != job.id);
                } else if let Some(existing) =
                    store.jobs.iter_mut().find(|existing| existing.id == job.id)
                {
                    *existing = job.clone();
                }
            }
        }

        self.clear_active_run(&job.id).await;
        self.save_store().await;
        self.arm_timer().await;

        self.get_job(&job.id)
            .await
            .ok_or_else(|| format!("job {} no longer exists after execution", job.id))
    }

    pub async fn list_jobs(&self, include_disabled: bool) -> Vec<CronJob> {
        let store = self.load_store().await;
        let mut jobs: Vec<CronJob> = if include_disabled {
            store.jobs
        } else {
            store.jobs.into_iter().filter(|job| job.enabled).collect()
        };

        jobs.sort_by_key(|job| job.state.next_run_at_ms.unwrap_or(i64::MAX));
        jobs
    }

    pub async fn list_job_views(&self, include_disabled: bool) -> Vec<CronJobDto> {
        let jobs = self.list_jobs(include_disabled).await;
        let active_guard = self.active_runs.read().await;
        jobs.into_iter()
            .map(|job| {
                let active = active_guard.get(&job.id).map(|run| run.snapshot.clone());
                Self::to_job_dto(&job, active)
            })
            .collect()
    }

    pub async fn get_job(&self, job_id: &str) -> Option<CronJobDto> {
        let store = self.load_store().await;
        let job = store.jobs.into_iter().find(|job| job.id == job_id)?;
        let active = self.active_snapshot_for(job_id).await;
        Some(Self::to_job_dto(&job, active))
    }

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

    pub async fn create_job(&self, request: CreateCronJobRequest) -> Result<CronJobDto, String> {
        let now = now_ms();
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let schedule = request.schedule.clone();
        let job = CronJob {
            id,
            name: request.name,
            enabled: request.enabled,
            schedule,
            payload: request.payload,
            state: CronJobState {
                next_run_at_ms: if request.enabled {
                    compute_next_run(&request.schedule, now)
                } else {
                    None
                },
                ..Default::default()
            },
            created_at_ms: now,
            updated_at_ms: now,
            delete_after_run: request.delete_after_run,
        };

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
        Ok(Self::to_job_dto(&job, None))
    }

    pub async fn update_job(
        &self,
        job_id: &str,
        request: UpdateCronJobRequest,
    ) -> Result<CronJobDto, String> {
        let updated_job = {
            let mut store_guard = self.store.write().await;
            let store = store_guard
                .as_mut()
                .ok_or_else(|| "cron store not initialized".to_string())?;
            let job = store
                .jobs
                .iter_mut()
                .find(|job| job.id == job_id)
                .ok_or_else(|| format!("job {} not found", job_id))?;

            job.name = request.name;
            job.schedule = request.schedule;
            job.payload = request.payload;
            job.delete_after_run = request.delete_after_run;
            job.enabled = request.enabled;
            job.updated_at_ms = now_ms();
            job.state.next_run_at_ms = if job.enabled {
                compute_next_run(&job.schedule, now_ms())
            } else {
                None
            };
            job.clone()
        };

        self.save_store().await;
        self.arm_timer().await;
        Ok(Self::to_job_dto(
            &updated_job,
            self.active_snapshot_for(job_id).await,
        ))
    }

    pub async fn delete_job(&self, job_id: &str) -> Result<(), String> {
        let _ = self.stop_run(job_id).await;
        let removed = {
            let mut store_guard = self.store.write().await;
            if let Some(store) = store_guard.as_mut() {
                let before = store.jobs.len();
                store.jobs.retain(|job| job.id != job_id);
                store.jobs.len() < before
            } else {
                false
            }
        };

        self.clear_active_run(job_id).await;

        if removed {
            self.save_store().await;
            self.arm_timer().await;
            Ok(())
        } else {
            Err(format!("job {} not found", job_id))
        }
    }

    pub async fn remove_job(&self, job_id: &str) -> bool {
        self.delete_job(job_id).await.is_ok()
    }

    pub async fn enable_job(&self, job_id: &str, enabled: bool) -> Option<CronJob> {
        let job = {
            let mut store_guard = self.store.write().await;
            if let Some(store) = store_guard.as_mut() {
                if let Some(job) = store.jobs.iter_mut().find(|job| job.id == job_id) {
                    job.enabled = enabled;
                    job.updated_at_ms = now_ms();
                    job.state.next_run_at_ms = if enabled {
                        compute_next_run(&job.schedule, now_ms())
                    } else {
                        None
                    };
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

    pub async fn set_job_enabled(&self, job_id: &str, enabled: bool) -> Result<CronJobDto, String> {
        let updated = self
            .enable_job(job_id, enabled)
            .await
            .ok_or_else(|| format!("job {} not found", job_id))?;
        Ok(Self::to_job_dto(
            &updated,
            self.active_snapshot_for(job_id).await,
        ))
    }

    pub async fn run_job(&self, job_id: &str, force: bool) -> bool {
        self.run_job_now(job_id, force).await.is_ok()
    }

    pub async fn run_job_now(&self, job_id: &str, force: bool) -> Result<CronJobDto, String> {
        let job = {
            let store_guard = self.store.read().await;
            if let Some(store) = store_guard.as_ref() {
                store
                    .jobs
                    .iter()
                    .find(|job| job.id == job_id)
                    .filter(|job| force || job.enabled)
                    .cloned()
            } else {
                None
            }
        }
        .ok_or_else(|| format!("job {} not found or disabled", job_id))?;

        self.execute_job_with_trigger(job, CronTrigger::Manual)
            .await
    }

    pub async fn stop_run(&self, job_id: &str) -> Result<CronRunSnapshot, String> {
        let active = {
            let active_guard = self.active_runs.read().await;
            active_guard.get(job_id).cloned()
        }
        .ok_or_else(|| format!("job {} is not running", job_id))?;

        active.cancel_token.cancel();
        Ok(active.snapshot)
    }

    pub async fn status(&self) -> serde_json::Value {
        let jobs = self.list_job_views(true).await;
        let is_running = *self.running.read().await;
        let next_wake = self.get_next_wake_ms().await;

        serde_json::json!({
            "enabled": is_running,
            "jobs": jobs.len(),
            "runningJobs": jobs.iter().filter(|job| job.is_running).count(),
            "nextWakeAtMs": next_wake,
        })
    }
}

struct CronServiceHandle {
    store: Arc<RwLock<Option<CronStore>>>,
    timer_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    running: Arc<RwLock<bool>>,
    on_job: Option<JobCallback>,
    store_path: PathBuf,
    active_runs: Arc<RwLock<HashMap<String, ActiveCronRun>>>,
}

impl CronServiceHandle {
    async fn execute_due_job(&self, mut job: CronJob) {
        info!("Cron: due job fired '{}' ({})", job.name, job.id);
        let mut active_guard = self.active_runs.write().await;
        if active_guard.contains_key(&job.id) {
            return;
        }

        let timestamp = now_ms();
        active_guard.insert(
            job.id.clone(),
            ActiveCronRun {
                snapshot: CronRunSnapshot {
                    run_id: uuid::Uuid::new_v4().to_string(),
                    job_id: job.id.clone(),
                    started_at_ms: timestamp,
                    last_heartbeat_at_ms: timestamp,
                    trigger: CronTrigger::Scheduled,
                    cancelable: true,
                },
                cancel_token: CancellationToken::new(),
            },
        );
        let cancel_token = active_guard
            .get(&job.id)
            .map(|run| run.cancel_token.clone())
            .expect("inserted active run");
        drop(active_guard);

        let result = if cancel_token.is_cancelled() {
            Err("job cancelled before start".to_string())
        } else if let Some(callback) = &self.on_job {
            match (callback)(job.clone(), cancel_token.clone()).await {
                Some(response) if cancel_token.is_cancelled() => Err("job cancelled".to_string()),
                Some(response) if response.to_ascii_lowercase().starts_with("error") => {
                    Err(response)
                }
                Some(_) | None => Ok(()),
            }
        } else if cancel_token.is_cancelled() {
            Err("job cancelled".to_string())
        } else {
            Ok(())
        };

        job.state.last_run_at_ms = Some(timestamp);
        job.updated_at_ms = now_ms();
        match result {
            Ok(()) => {
                job.state.last_status = Some("ok".to_string());
                job.state.last_error = None;
                info!(
                    "Cron: scheduled job '{}' ({}) completed successfully",
                    job.name, job.id
                );
            }
            Err(err) => {
                job.state.last_status = Some("error".to_string());
                job.state.last_error = Some(err);
                warn!("Cron: scheduled job '{}' ({}) failed", job.name, job.id);
            }
        }

        let should_remove = match &job.schedule {
            CronSchedule::At { .. } => {
                if job.delete_after_run && job.state.last_status.as_deref() == Some("ok") {
                    true
                } else {
                    job.enabled = false;
                    job.state.next_run_at_ms = None;
                    false
                }
            }
            _ => {
                job.state.next_run_at_ms = if job.enabled {
                    compute_next_run(&job.schedule, now_ms())
                } else {
                    None
                };
                false
            }
        };

        {
            let mut store_guard = self.store.write().await;
            if let Some(store) = store_guard.as_mut() {
                if should_remove {
                    store.jobs.retain(|existing| existing.id != job.id);
                } else if let Some(existing) =
                    store.jobs.iter_mut().find(|existing| existing.id == job.id)
                {
                    *existing = job.clone();
                }
            }
        }

        let mut active_guard = self.active_runs.write().await;
        active_guard.remove(&job.id);
    }

    async fn on_timer(&self) {
        let now = now_ms();
        let due_jobs = {
            let store_guard = self.store.read().await;
            if let Some(store) = store_guard.as_ref() {
                store
                    .jobs
                    .iter()
                    .filter(|job| {
                        job.enabled
                            && job.state.next_run_at_ms.is_some()
                            && now >= job.state.next_run_at_ms.unwrap()
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        };

        for job in due_jobs {
            self.execute_due_job(job).await;
        }

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

        self.rearm_timer();
    }

    fn rearm_timer(&self) {
        let store = Arc::clone(&self.store);
        let timer_task = Arc::clone(&self.timer_task);
        let running = Arc::clone(&self.running);
        let on_job = self.on_job.clone();
        let store_path = self.store_path.clone();
        let active_runs = Arc::clone(&self.active_runs);

        tokio::spawn(async move {
            {
                let mut timer_guard = timer_task.lock().await;
                if let Some(task) = timer_guard.take() {
                    task.abort();
                }
            }

            let is_running = *running.read().await;
            if !is_running {
                return;
            }

            let next_wake = {
                let store_guard = store.read().await;
                store_guard.as_ref().and_then(|cron_store| {
                    cron_store
                        .jobs
                        .iter()
                        .filter(|job| job.enabled && job.state.next_run_at_ms.is_some())
                        .filter_map(|job| job.state.next_run_at_ms)
                        .min()
                })
            };

            let Some(next_wake) = next_wake else {
                return;
            };

            let delay_ms = (next_wake - now_ms()).max(0);
            let delay = tokio::time::Duration::from_millis(delay_ms as u64);
            let service = Arc::new(CronServiceHandle {
                store: Arc::clone(&store),
                timer_task: Arc::clone(&timer_task),
                running: Arc::clone(&running),
                on_job: on_job.clone(),
                store_path: store_path.clone(),
                active_runs: Arc::clone(&active_runs),
            });

            let task = tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                let is_running = *service.running.read().await;
                if is_running {
                    service.on_timer().await;
                }
            });

            let mut timer_guard = timer_task.lock().await;
            *timer_guard = Some(task);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compute_next_run_every() {
        let schedule = CronSchedule::every(5000);
        assert_eq!(compute_next_run(&schedule, 1000), Some(6000));
    }

    #[test]
    fn test_compute_next_run_cron_five_fields_supported() {
        let schedule = CronSchedule::cron("* * * * *".to_string(), None);
        let next = compute_next_run(&schedule, 0).unwrap();
        assert_eq!(next, 60_000);
    }

    #[tokio::test]
    async fn test_enable_disable_updates_status() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        let service = CronService::new(store_path, None);
        service.start().await;

        let job = service
            .create_job(CreateCronJobRequest {
                name: "Test".to_string(),
                schedule: CronSchedule::every(5000),
                payload: CronPayload::default(),
                delete_after_run: false,
                enabled: true,
            })
            .await
            .unwrap();

        let paused = service.set_job_enabled(&job.job.id, false).await.unwrap();
        assert_eq!(paused.computed_status, CronJobLifecycleStatus::Paused);

        let scheduled = service.set_job_enabled(&job.job.id, true).await.unwrap();
        assert_eq!(scheduled.computed_status, CronJobLifecycleStatus::Scheduled);
        service.stop().await;
    }

    #[tokio::test]
    async fn test_delete_running_job_removes_it() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        let callback: JobCallback = Arc::new(|_job, token| {
            Box::pin(async move {
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => Some("done".to_string()),
                    _ = token.cancelled() => Some("Error: cancelled".to_string()),
                }
            })
        });
        let service = Arc::new(CronService::new(store_path, Some(callback)));
        service.start().await;

        let job = service
            .create_job(CreateCronJobRequest {
                name: "Long".to_string(),
                schedule: CronSchedule::every(5000),
                payload: CronPayload::default(),
                delete_after_run: false,
                enabled: true,
            })
            .await
            .unwrap();

        let job_id = job.job.id.clone();
        let service_for_run = Arc::clone(&service);
        let job_id_for_run = job_id.clone();
        let runner = tokio::spawn(async move {
            let _ = service_for_run.run_job_now(&job_id_for_run, true).await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        service.delete_job(&job_id).await.unwrap();
        let jobs = service.list_job_views(true).await;
        assert!(jobs.into_iter().all(|job| job.job.id != job_id));

        let _ = runner.await;
        service.stop().await;
    }
}
