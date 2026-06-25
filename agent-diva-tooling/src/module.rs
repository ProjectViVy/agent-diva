//! Runtime module lifecycle primitives and registrations.

use agent_diva_core::bus::MessageBus;
use agent_diva_core::config::Config;
use agent_diva_core::cron::CronService;
use agent_diva_core::heartbeat::types::HeartbeatConfig;
use agent_diva_core::heartbeat::HeartbeatService;
use agent_diva_core::presence::{PresenceConfig, PresenceState};
use agent_diva_core::security::SharedSecurityPolicy;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Shared services injected into each running module.
#[derive(Clone)]
pub struct ModuleCtx {
    pub bus: Arc<MessageBus>,
    pub security: SharedSecurityPolicy,
    pub config: Arc<Config>,
    pub presence: Arc<RwLock<PresenceState>>,
}

/// Extra runtime construction inputs needed before `start()`.
#[derive(Clone)]
pub struct ModuleBuildContext {
    pub module_ctx: ModuleCtx,
    pub workspace: PathBuf,
    pub cron_store: PathBuf,
    pub cron_service: Option<Arc<CronService>>,
    pub heartbeat_service: Option<Arc<HeartbeatService>>,
}

/// Long-lived runtime service lifecycle.
#[async_trait]
pub trait Module: Send + Sync {
    fn name(&self) -> &str;

    async fn start(&self, ctx: &ModuleCtx) -> Result<()>;

    async fn stop(&self) -> Result<()>;

    fn dependencies(&self) -> Vec<&str> {
        Vec::new()
    }

    fn on_config_reload(&self, _new_config: &Config) -> Result<()> {
        Ok(())
    }
}

/// Inventory registration for runtime modules.
pub struct ModuleRegistration {
    pub key: &'static str,
    pub constructor: fn(&ModuleBuildContext) -> Result<Arc<dyn Module>>,
}

inventory::collect!(ModuleRegistration);

/// Started modules in topological order.
pub struct ModuleStartup {
    started: Vec<Arc<dyn Module>>,
}

impl ModuleStartup {
    pub fn from_inventory(build_ctx: &ModuleBuildContext) -> Result<Self> {
        let mut modules = Vec::new();
        for registration in inventory::iter::<ModuleRegistration> {
            modules.push((registration.constructor)(build_ctx)?);
        }
        let started = topological_sort(modules)?;
        Ok(Self { started })
    }

    pub async fn start_all(&self, ctx: &ModuleCtx) -> Result<()> {
        for module in &self.started {
            info!("starting module {}", module.name());
            module.start(ctx).await?;
        }
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<()> {
        for module in self.started.iter().rev() {
            info!("stopping module {}", module.name());
            module.stop().await?;
        }
        Ok(())
    }

    pub fn module_names(&self) -> Vec<String> {
        self.started
            .iter()
            .map(|module| module.name().to_string())
            .collect()
    }
}

fn topological_sort(modules: Vec<Arc<dyn Module>>) -> Result<Vec<Arc<dyn Module>>> {
    let mut nodes = HashMap::new();
    for module in modules {
        let name = module.name().to_string();
        if nodes.insert(name.clone(), module).is_some() {
            return Err(anyhow!("duplicate module registration: {}", name));
        }
    }

    let mut indegree = HashMap::<String, usize>::new();
    let mut edges = HashMap::<String, Vec<String>>::new();
    for (name, module) in &nodes {
        indegree.entry(name.clone()).or_insert(0);
        for dependency in module.dependencies() {
            if !nodes.contains_key(dependency) {
                return Err(anyhow!(
                    "module {} depends on unknown module {}",
                    name,
                    dependency
                ));
            }
            indegree
                .entry(name.clone())
                .and_modify(|value| *value += 1)
                .or_insert(1);
            edges
                .entry(dependency.to_string())
                .or_default()
                .push(name.clone());
        }
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(name, degree)| (*degree == 0).then_some(name.clone()))
        .collect::<Vec<_>>();
    ready.sort();
    let mut queue = ready.into_iter().collect::<VecDeque<_>>();
    let mut ordered = Vec::new();

    while let Some(name) = queue.pop_front() {
        ordered.push(
            nodes
                .get(&name)
                .cloned()
                .ok_or_else(|| anyhow!("module {} disappeared during sort", name))?,
        );
        if let Some(dependents) = edges.get(&name) {
            let mut new_ready = Vec::new();
            for dependent in dependents {
                let degree = indegree
                    .get_mut(dependent)
                    .ok_or_else(|| anyhow!("missing indegree for module {}", dependent))?;
                *degree -= 1;
                if *degree == 0 {
                    new_ready.push(dependent.clone());
                }
            }
            new_ready.sort();
            for dependent in new_ready {
                queue.push_back(dependent);
            }
        }
    }

    if ordered.len() != nodes.len() {
        return Err(anyhow!("module dependency cycle detected"));
    }

    Ok(ordered)
}

/// Tracks lightweight presence transitions from bus activity.
pub struct PresenceService {
    config: PresenceConfig,
    running: Arc<RwLock<bool>>,
    last_seen: Arc<RwLock<Instant>>,
    activity_task: Mutex<Option<JoinHandle<()>>>,
    transition_task: Mutex<Option<JoinHandle<()>>>,
}

impl Default for PresenceService {
    fn default() -> Self {
        Self {
            config: PresenceConfig::default(),
            running: Arc::new(RwLock::new(false)),
            last_seen: Arc::new(RwLock::new(Instant::now())),
            activity_task: Mutex::new(None),
            transition_task: Mutex::new(None),
        }
    }
}

#[async_trait]
impl Module for PresenceService {
    fn name(&self) -> &str {
        "presence"
    }

    async fn start(&self, ctx: &ModuleCtx) -> Result<()> {
        {
            let running = self.running.read().await;
            if *running {
                debug!("presence module already running");
                return Ok(());
            }
        }

        *self.running.write().await = true;
        *ctx.presence.write().await = PresenceState::Active;
        *self.last_seen.write().await = Instant::now();

        let running = Arc::clone(&self.running);
        let last_seen = Arc::clone(&self.last_seen);
        let presence = Arc::clone(&ctx.presence);
        let activity_presence = Arc::clone(&ctx.presence);
        let active_timeout = self.config.active_timeout_s;
        let distracted_timeout = self.config.distracted_timeout_s;
        let mut event_rx = ctx.bus.subscribe_events();

        let activity_task = tokio::spawn(async move {
            loop {
                if !*running.read().await {
                    break;
                }

                match event_rx.recv().await {
                    Ok(_) => {
                        *last_seen.write().await = Instant::now();
                        let mut state = activity_presence.write().await;
                        if *state != PresenceState::Active {
                            *state = PresenceState::Active;
                        }
                    }
                    Err(error) => {
                        warn!("presence activity listener stopped: {}", error);
                        break;
                    }
                }
            }
        });

        let running = Arc::clone(&self.running);
        let last_seen = Arc::clone(&self.last_seen);
        let transition_task = tokio::spawn(async move {
            let tick = std::time::Duration::from_secs(1);
            loop {
                tokio::time::sleep(tick).await;
                if !*running.read().await {
                    break;
                }

                let elapsed = last_seen.read().await.elapsed().as_secs();
                let next = if elapsed >= distracted_timeout {
                    PresenceState::Gone
                } else if elapsed >= active_timeout {
                    PresenceState::Distracted
                } else {
                    PresenceState::Active
                };

                let mut state = presence.write().await;
                if *state != next {
                    *state = next;
                }
            }
        });

        *self.activity_task.lock().await = Some(activity_task);
        *self.transition_task.lock().await = Some(transition_task);
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.running.write().await = false;

        if let Some(task) = self.activity_task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
        if let Some(task) = self.transition_task.lock().await.take() {
            task.abort();
            let _ = task.await;
        }

        Ok(())
    }
}

/// Security lifecycle hook for runtime policy state.
#[derive(Default)]
pub struct SafetyService {
    running: Arc<RwLock<bool>>,
}

#[async_trait]
impl Module for SafetyService {
    fn name(&self) -> &str {
        "safety"
    }

    async fn start(&self, ctx: &ModuleCtx) -> Result<()> {
        *self.running.write().await = true;
        debug!(
            "safety module active at security level {:?}",
            ctx.security.config().level
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.running.write().await = false;
        Ok(())
    }
}

/// Sandbox lifecycle hook anchored to the shared security policy.
#[derive(Default)]
pub struct SandboxService {
    running: Arc<RwLock<bool>>,
}

#[async_trait]
impl Module for SandboxService {
    fn name(&self) -> &str {
        "sandbox"
    }

    async fn start(&self, ctx: &ModuleCtx) -> Result<()> {
        *self.running.write().await = true;
        debug!(
            "sandbox module active; shell access allowed: {}",
            ctx.security.has_shell_access()
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.running.write().await = false;
        Ok(())
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["safety"]
    }
}

#[async_trait]
impl Module for CronService {
    fn name(&self) -> &str {
        "cron"
    }

    async fn start(&self, _ctx: &ModuleCtx) -> Result<()> {
        CronService::start(self).await;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        CronService::stop(self).await;
        Ok(())
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["sandbox"]
    }
}

#[async_trait]
impl Module for HeartbeatService {
    fn name(&self) -> &str {
        "heartbeat"
    }

    async fn start(&self, _ctx: &ModuleCtx) -> Result<()> {
        HeartbeatService::start(self).await;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        HeartbeatService::stop(self).await;
        Ok(())
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["presence"]
    }
}

fn build_presence_module(_ctx: &ModuleBuildContext) -> Result<Arc<dyn Module>> {
    Ok(Arc::new(PresenceService::default()))
}

fn build_safety_module(_ctx: &ModuleBuildContext) -> Result<Arc<dyn Module>> {
    Ok(Arc::new(SafetyService::default()))
}

fn build_sandbox_module(_ctx: &ModuleBuildContext) -> Result<Arc<dyn Module>> {
    Ok(Arc::new(SandboxService::default()))
}

fn build_cron_module(ctx: &ModuleBuildContext) -> Result<Arc<dyn Module>> {
    ctx.cron_service
        .clone()
        .map(|module| module as Arc<dyn Module>)
        .ok_or_else(|| anyhow!("cron module requires a prebuilt cron service"))
}

fn build_heartbeat_module(ctx: &ModuleBuildContext) -> Result<Arc<dyn Module>> {
    let service = ctx.heartbeat_service.clone().unwrap_or_else(|| {
        Arc::new(HeartbeatService::new(
            ctx.workspace.clone(),
            HeartbeatConfig::default(),
            Some((*ctx.module_ctx.bus).clone()),
            None,
            None,
        ))
    });
    Ok(service as Arc<dyn Module>)
}

inventory::submit! {
    ModuleRegistration {
        key: "presence",
        constructor: build_presence_module,
    }
}

inventory::submit! {
    ModuleRegistration {
        key: "safety",
        constructor: build_safety_module,
    }
}

inventory::submit! {
    ModuleRegistration {
        key: "sandbox",
        constructor: build_sandbox_module,
    }
}

inventory::submit! {
    ModuleRegistration {
        key: "cron",
        constructor: build_cron_module,
    }
}

inventory::submit! {
    ModuleRegistration {
        key: "heartbeat",
        constructor: build_heartbeat_module,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::security::SecurityPolicy;

    struct TestModule {
        name: &'static str,
        dependencies: Vec<&'static str>,
    }

    #[async_trait]
    impl Module for TestModule {
        fn name(&self) -> &str {
            self.name
        }

        async fn start(&self, _ctx: &ModuleCtx) -> Result<()> {
            Ok(())
        }

        async fn stop(&self) -> Result<()> {
            Ok(())
        }

        fn dependencies(&self) -> Vec<&str> {
            self.dependencies.clone()
        }
    }

    #[test]
    fn topological_sort_orders_dependencies_first() {
        let modules: Vec<Arc<dyn Module>> = vec![
            Arc::new(TestModule {
                name: "heartbeat",
                dependencies: vec!["presence"],
            }),
            Arc::new(TestModule {
                name: "presence",
                dependencies: vec![],
            }),
            Arc::new(TestModule {
                name: "sandbox",
                dependencies: vec!["safety"],
            }),
            Arc::new(TestModule {
                name: "safety",
                dependencies: vec![],
            }),
        ];

        let order = topological_sort(modules)
            .unwrap()
            .into_iter()
            .map(|module| module.name().to_string())
            .collect::<Vec<_>>();

        let heartbeat_index = order.iter().position(|name| name == "heartbeat").unwrap();
        let presence_index = order.iter().position(|name| name == "presence").unwrap();
        let sandbox_index = order.iter().position(|name| name == "sandbox").unwrap();
        let safety_index = order.iter().position(|name| name == "safety").unwrap();

        assert!(presence_index < heartbeat_index);
        assert!(safety_index < sandbox_index);
    }

    #[test]
    fn topological_sort_rejects_cycles() {
        let modules: Vec<Arc<dyn Module>> = vec![
            Arc::new(TestModule {
                name: "a",
                dependencies: vec!["b"],
            }),
            Arc::new(TestModule {
                name: "b",
                dependencies: vec!["a"],
            }),
        ];

        assert!(topological_sort(modules).is_err());
    }

    #[tokio::test]
    async fn module_startup_collects_inventory_modules() {
        let bus = Arc::new(MessageBus::new());
        let config = Arc::new(Config::default());
        let security = Arc::new(SecurityPolicy::new(std::env::temp_dir()));
        let presence = Arc::new(RwLock::new(PresenceState::Active));
        let module_ctx = ModuleCtx {
            bus,
            security,
            config,
            presence,
        };
        let build_ctx = ModuleBuildContext {
            module_ctx,
            workspace: std::env::temp_dir(),
            cron_store: std::env::temp_dir().join("cron.json"),
            cron_service: Some(Arc::new(CronService::new(
                std::env::temp_dir().join("cron.json"),
                None,
            ))),
            heartbeat_service: None,
        };

        let startup = ModuleStartup::from_inventory(&build_ctx).unwrap();
        let names = startup.module_names();
        assert!(names.iter().any(|name| name == "presence"));
        assert!(names.iter().any(|name| name == "heartbeat"));
        assert!(names.iter().any(|name| name == "cron"));
    }
}
