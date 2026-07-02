//! Module trait and lifecycle context for the modular runtime.
//!
//! Defines the `Module` trait that all system services must implement,
//! and the `ModuleCtx` struct that provides dependency injection through
//! shared `Arc`-wrapped services.

use agent_diva_core::bus::MessageBus;
use agent_diva_core::config::Config;
use agent_diva_core::presence::PresenceState;
use agent_diva_core::security::SharedSecurityPolicy;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared context injected into every module at startup.
///
/// Provides access to the event bus, security policy, configuration,
/// and user presence state — all behind `Arc` for shared ownership.
#[derive(Clone)]
pub struct ModuleCtx {
    /// Event bus for inter-module communication.
    pub bus: Arc<MessageBus>,
    /// Security policy for PII/injection/tool authorization.
    pub security: SharedSecurityPolicy,
    /// Current system configuration (wrapped for hot-reload support).
    pub config: Arc<RwLock<Config>>,
    /// User presence state (Active / Distracted / Gone).
    pub presence: Arc<RwLock<PresenceState>>,
}

impl ModuleCtx {
    /// Create a new `ModuleCtx` with all four `Arc`-wrapped services.
    pub fn new(
        bus: Arc<MessageBus>,
        security: SharedSecurityPolicy,
        config: Arc<RwLock<Config>>,
        presence: Arc<RwLock<PresenceState>>,
    ) -> Self {
        Self {
            bus,
            security,
            config,
            presence,
        }
    }
}

/// Standard lifecycle for all system modules.
///
/// Implementations must be `Send + Sync + 'static` so they can be held
/// as `Arc<dyn Module>` and started / stopped by the bootstrap system.
#[async_trait]
pub trait Module: Send + Sync + 'static {
    /// Unique module identifier (e.g. `"mcp_service"`, `"cron_service"`).
    fn name(&self) -> &str;

    /// Start the module with the injected context.
    ///
    /// Called once during bootstrap after all dependencies have started.
    async fn start(&self, ctx: ModuleCtx) -> crate::Result<()>;

    /// Gracefully stop the module.
    ///
    /// Called during shutdown in reverse dependency order.
    async fn stop(&self) -> crate::Result<()>;

    /// Declare dependencies on other modules by name.
    ///
    /// Used by the bootstrap topological sorter to determine start order.
    /// Default: no dependencies.
    fn dependencies(&self) -> &[&str] {
        &[]
    }

    /// Called when configuration is hot-reloaded.
    ///
    /// Modules should update their internal state if the change affects
    /// them. Default: no-op.
    async fn on_config_reload(&self, _new_config: &Config) -> crate::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
//  Bootstrap types — ModuleRegistry, Bootstrap, BootstrapError
// ---------------------------------------------------------------------------

use std::collections::{HashMap, VecDeque};
use tracing::{error, info};

/// Errors that can occur during module bootstrap.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    /// A module declared a dependency on a module that is not registered.
    #[error("Missing dependency: module '{module}' requires '{dependency}'")]
    MissingDependency { module: String, dependency: String },
    /// A circular dependency was detected among modules.
    #[error("Circular dependency detected: {cycle:?}")]
    CycleDetected { cycle: Vec<String> },
    /// A module failed to start.
    #[error("Module '{module}' failed to start: {error}")]
    StartFailed { module: String, error: String },
}

/// Collection of registered modules with dependency-aware ordering.
#[derive(Default)]
pub struct ModuleRegistry {
    modules: Vec<Arc<dyn Module>>,
}

impl ModuleRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    /// Register a module.
    pub fn register(&mut self, module: Arc<dyn Module>) {
        self.modules.push(module);
    }

    /// Get a module by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Module>> {
        self.modules.iter().find(|m| m.name() == name)
    }

    /// Get all registered modules.
    pub fn all(&self) -> &[Arc<dyn Module>] {
        &self.modules
    }

    /// Number of registered modules.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Resolve start order using Kahn's algorithm (topological sort).
    ///
    /// Returns modules sorted so that dependencies come first.
    /// Detects missing dependencies and circular dependencies.
    pub fn resolve_start_order(&self) -> std::result::Result<Vec<Arc<dyn Module>>, BootstrapError> {
        let name_to_idx: HashMap<&str, usize> = self
            .modules
            .iter()
            .enumerate()
            .map(|(i, m)| (m.name(), i))
            .collect();

        // Build adjacency list: for each dependency, list its dependents
        let mut in_degree = vec![0usize; self.modules.len()];
        let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::new();

        for (i, module) in self.modules.iter().enumerate() {
            for dep_name in module.dependencies() {
                let dep_idx = *name_to_idx.get(dep_name).ok_or_else(|| {
                    BootstrapError::MissingDependency {
                        module: module.name().to_string(),
                        dependency: dep_name.to_string(),
                    }
                })?;
                adjacency.entry(dep_idx).or_default().push(i);
                in_degree[i] += 1;
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<usize> = in_degree
            .iter()
            .enumerate()
            .filter(|(_, &deg)| deg == 0)
            .map(|(i, _)| i)
            .collect();

        let mut sorted: Vec<Arc<dyn Module>> = Vec::with_capacity(self.modules.len());

        while let Some(idx) = queue.pop_front() {
            sorted.push(Arc::clone(&self.modules[idx]));
            if let Some(dependents) = adjacency.get(&idx) {
                for &dep_idx in dependents {
                    in_degree[dep_idx] -= 1;
                    if in_degree[dep_idx] == 0 {
                        queue.push_back(dep_idx);
                    }
                }
            }
        }

        if sorted.len() != self.modules.len() {
            // The remaining modules form a cycle
            let cycle: Vec<String> = in_degree
                .iter()
                .enumerate()
                .filter(|(_, &deg)| deg > 0)
                .map(|(i, _)| self.modules[i].name().to_string())
                .collect();
            return Err(BootstrapError::CycleDetected { cycle });
        }

        Ok(sorted)
    }

    /// Resolve shutdown order (reverse of start order).
    pub fn resolve_shutdown_order(
        &self,
    ) -> std::result::Result<Vec<Arc<dyn Module>>, BootstrapError> {
        let mut order = self.resolve_start_order()?;
        order.reverse();
        Ok(order)
    }
}

/// Executes the module lifecycle: start all in dependency order,
/// or rollback on failure.
pub struct Bootstrap {
    registry: ModuleRegistry,
}

impl Bootstrap {
    /// Create a new bootstrap runner from a populated registry.
    pub fn new(registry: ModuleRegistry) -> Self {
        Self { registry }
    }

    /// Start all modules in dependency order.
    ///
    /// On failure, stops already-started modules in reverse order
    /// and returns the error.
    pub async fn start_all(&self, ctx: ModuleCtx) -> std::result::Result<(), BootstrapError> {
        let order = self.registry.resolve_start_order()?;
        let mut started: Vec<Arc<dyn Module>> = Vec::with_capacity(order.len());

        for module in &order {
            info!("Starting module: {}", module.name());
            match module.start(ctx.clone()).await {
                Ok(()) => {
                    started.push(Arc::clone(module));
                }
                Err(e) => {
                    error!("Module '{}' failed to start: {}", module.name(), e);
                    // Rollback: stop already-started modules in reverse order
                    for started_mod in started.iter().rev() {
                        if let Err(stop_err) = started_mod.stop().await {
                            error!(
                                "Error stopping '{}' during rollback: {}",
                                started_mod.name(),
                                stop_err
                            );
                        }
                    }
                    return Err(BootstrapError::StartFailed {
                        module: module.name().to_string(),
                        error: e.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Stop all modules in reverse dependency order.
    ///
    /// Errors during stop are logged but do not prevent subsequent
    /// modules from being stopped.
    pub async fn stop_all(&self) {
        let order = match self.registry.resolve_shutdown_order() {
            Ok(o) => o,
            Err(e) => {
                error!("Failed to resolve shutdown order: {}", e);
                return;
            }
        };

        for module in &order {
            info!("Stopping module: {}", module.name());
            if let Err(e) = module.stop().await {
                error!("Error stopping '{}': {}", module.name(), e);
            }
        }
    }

    /// Get a reference to the underlying registry.
    pub fn registry(&self) -> &ModuleRegistry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::security::SecurityPolicy;
    use std::path::PathBuf;

    struct TestModule {
        started: std::sync::atomic::AtomicBool,
        stopped: std::sync::atomic::AtomicBool,
    }

    #[async_trait]
    impl Module for TestModule {
        fn name(&self) -> &str {
            "test_module"
        }

        async fn start(&self, _ctx: ModuleCtx) -> crate::Result<()> {
            self.started
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        async fn stop(&self) -> crate::Result<()> {
            self.stopped
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    /// AC1 + AC3: Module trait satisfies Send + Sync + 'static and
    /// can be stored as `Box<dyn Module>`.
    #[test]
    fn test_module_trait_bounds() {
        let m = TestModule {
            started: std::sync::atomic::AtomicBool::new(false),
            stopped: std::sync::atomic::AtomicBool::new(false),
        };
        // Verify it compiles as Box<dyn Module>
        let _boxed: Box<dyn Module> = Box::new(m);

        fn assert_send_sync<T: Send + Sync + 'static>() {}
        assert_send_sync::<TestModule>();
    }

    /// AC2: ModuleCtx creation with all 4 Arcs.
    #[test]
    fn test_module_ctx_creation() {
        let bus = Arc::new(MessageBus::new());
        let security: SharedSecurityPolicy = Arc::new(SecurityPolicy::new(PathBuf::from("/tmp")));
        let config = Arc::new(RwLock::new(Config::default()));
        let presence = Arc::new(RwLock::new(PresenceState::default()));

        let ctx = ModuleCtx::new(bus, security, config, presence);
        // Verify all 4 fields are accessible
        let _ = Arc::clone(&ctx.bus);
        let _ = Arc::clone(&ctx.security);
        let _ = Arc::clone(&ctx.config);
        let _ = Arc::clone(&ctx.presence);
    }

    /// AC4: Default implementations for dependencies() and on_config_reload().
    #[tokio::test]
    async fn test_module_defaults() {
        let m = TestModule {
            started: std::sync::atomic::AtomicBool::new(false),
            stopped: std::sync::atomic::AtomicBool::new(false),
        };

        // dependencies() should return empty slice by default
        assert!(m.dependencies().is_empty());

        // on_config_reload() should return Ok(()) by default
        let result = m.on_config_reload(&Config::default()).await;
        assert!(result.is_ok());
    }

    /// AC5 + lifecycle: Module start and stop with state transitions.
    #[tokio::test]
    async fn test_module_lifecycle() {
        let m = TestModule {
            started: std::sync::atomic::AtomicBool::new(false),
            stopped: std::sync::atomic::AtomicBool::new(false),
        };

        assert!(!m.started.load(std::sync::atomic::Ordering::SeqCst));
        assert!(!m.stopped.load(std::sync::atomic::Ordering::SeqCst));

        let bus = Arc::new(MessageBus::new());
        let security: SharedSecurityPolicy = Arc::new(SecurityPolicy::new(PathBuf::from("/tmp")));
        let config = Arc::new(RwLock::new(Config::default()));
        let presence = Arc::new(RwLock::new(PresenceState::default()));
        let ctx = ModuleCtx::new(bus, security, config, presence);

        // Start
        m.start(ctx).await.unwrap();
        assert!(m.started.load(std::sync::atomic::Ordering::SeqCst));
        assert!(!m.stopped.load(std::sync::atomic::Ordering::SeqCst));

        // Stop
        m.stop().await.unwrap();
        assert!(m.stopped.load(std::sync::atomic::Ordering::SeqCst));
    }

    // ------------------------------------------------------------------
    //  Story 3.2 — ModuleRegistry + Bootstrap tests
    // ------------------------------------------------------------------

    /// Helper: module with configurable name and dependencies.
    struct NamedModule {
        name: &'static str,
        deps: &'static [&'static str],
        started: std::sync::atomic::AtomicBool,
        stopped: std::sync::atomic::AtomicBool,
        fail_on_start: std::sync::atomic::AtomicBool,
        fail_on_stop: std::sync::atomic::AtomicBool,
    }

    impl NamedModule {
        fn new(name: &'static str, deps: &'static [&'static str]) -> Self {
            Self {
                name,
                deps,
                started: std::sync::atomic::AtomicBool::new(false),
                stopped: std::sync::atomic::AtomicBool::new(false),
                fail_on_start: std::sync::atomic::AtomicBool::new(false),
                fail_on_stop: std::sync::atomic::AtomicBool::new(false),
            }
        }

        fn with_fail_on_start(self) -> Self {
            self.fail_on_start
                .store(true, std::sync::atomic::Ordering::SeqCst);
            self
        }

        fn is_started(&self) -> bool {
            self.started.load(std::sync::atomic::Ordering::SeqCst)
        }

        fn is_stopped(&self) -> bool {
            self.stopped.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl Module for NamedModule {
        fn name(&self) -> &str {
            self.name
        }

        fn dependencies(&self) -> &[&str] {
            self.deps
        }

        async fn start(&self, _ctx: ModuleCtx) -> crate::Result<()> {
            if self.fail_on_start.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(crate::ToolError::Error(format!(
                    "{} failed to start",
                    self.name
                )));
            }
            self.started
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        async fn stop(&self) -> crate::Result<()> {
            if self.fail_on_stop.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(crate::ToolError::Error(format!(
                    "{} failed to stop",
                    self.name
                )));
            }
            self.stopped
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    fn make_ctx() -> ModuleCtx {
        let bus = Arc::new(MessageBus::new());
        let security: SharedSecurityPolicy = Arc::new(SecurityPolicy::new(PathBuf::from("/tmp")));
        let config = Arc::new(RwLock::new(Config::default()));
        let presence = Arc::new(RwLock::new(PresenceState::default()));
        ModuleCtx::new(bus, security, config, presence)
    }

    /// Helper: create a NamedModule as `Arc<dyn Module>`.
    fn make_module(name: &'static str, deps: &'static [&'static str]) -> Arc<dyn Module> {
        Arc::new(NamedModule::new(name, deps))
    }

    /// Empty registry produces empty start order.
    #[test]
    fn test_empty_registry() {
        let registry = ModuleRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.get("anything").is_none());

        let order = registry.resolve_start_order().unwrap();
        assert!(order.is_empty());

        let shutdown = registry.resolve_shutdown_order().unwrap();
        assert!(shutdown.is_empty());
    }

    /// One module with no deps returns just that module.
    #[test]
    fn test_single_module() {
        let mut registry = ModuleRegistry::new();
        let m = make_module("a", &[]);
        registry.register(Arc::clone(&m));

        let order = registry.resolve_start_order().unwrap();
        assert_eq!(order.len(), 1);
        assert_eq!(order[0].name(), "a");

        let shutdown = registry.resolve_shutdown_order().unwrap();
        assert_eq!(shutdown.len(), 1);
    }

    /// Linear deps A→B→C returns [A, B, C] start, [C, B, A] shutdown.
    #[test]
    fn test_linear_deps() {
        let mut registry = ModuleRegistry::new();
        let a = make_module("a", &[]);
        let b = make_module("b", &["a"]);
        let c = make_module("c", &["b"]);
        registry.register(Arc::clone(&a));
        registry.register(Arc::clone(&b));
        registry.register(Arc::clone(&c));

        let order = registry.resolve_start_order().unwrap();
        let names: Vec<&str> = order.iter().map(|m| m.name()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);

        let shutdown = registry.resolve_shutdown_order().unwrap();
        let s_names: Vec<&str> = shutdown.iter().map(|m| m.name()).collect();
        assert_eq!(s_names, vec!["c", "b", "a"]);
    }

    /// Diamond deps: A→B, A→C, (B,C)→D.
    /// Valid order: A before B/C, B/C before D.
    #[test]
    fn test_diamond_deps() {
        let mut registry = ModuleRegistry::new();
        let a = make_module("a", &[]);
        let b = make_module("b", &["a"]);
        let c = make_module("c", &["a"]);
        let d = make_module("d", &["b", "c"]);
        registry.register(Arc::clone(&a));
        registry.register(Arc::clone(&b));
        registry.register(Arc::clone(&c));
        registry.register(Arc::clone(&d));

        let order = registry.resolve_start_order().unwrap();
        let names: Vec<&str> = order.iter().map(|m| m.name()).collect();

        // A must be first
        assert_eq!(names[0], "a");
        // D must be last
        assert_eq!(names[3], "d");
        // B and C must come before D
        let pos_b = names.iter().position(|&n| n == "b").unwrap();
        let pos_c = names.iter().position(|&n| n == "c").unwrap();
        let pos_d = names.iter().position(|&n| n == "d").unwrap();
        assert!(pos_b < pos_d);
        assert!(pos_c < pos_d);
    }

    /// Cycle detection: A→B→A returns CycleDetected.
    #[test]
    fn test_cycle_detection() {
        let mut registry = ModuleRegistry::new();
        let a = make_module("a", &["b"]);
        let b = make_module("b", &["a"]);
        registry.register(Arc::clone(&a));
        registry.register(Arc::clone(&b));

        assert!(
            matches!(
                registry.resolve_start_order(),
                Err(BootstrapError::CycleDetected { .. })
            ),
            "expected CycleDetected error"
        );
    }

    /// Missing dependency: A→B where B not registered.
    #[test]
    fn test_missing_dep() {
        let mut registry = ModuleRegistry::new();
        let a = make_module("a", &["missing_b"]);
        registry.register(Arc::clone(&a));

        match registry.resolve_start_order() {
            Err(BootstrapError::MissingDependency { module, dependency }) => {
                assert_eq!(module, "a");
                assert_eq!(dependency, "missing_b");
            }
            other => panic!(
                "expected MissingDependency, got: {:?}",
                other.as_ref().err()
            ),
        }
    }

    /// Start failure triggers rollback — already-started modules get stopped.
    #[tokio::test]
    async fn test_start_failure_rollback() {
        let mut registry = ModuleRegistry::new();
        let a_named = Arc::new(NamedModule::new("a", &[]));
        let b_named = Arc::new(NamedModule::new("b", &["a"]));
        let c_named = Arc::new(NamedModule::new("c", &["b"]).with_fail_on_start());
        registry.register(Arc::clone(&a_named) as Arc<dyn Module>);
        registry.register(Arc::clone(&b_named) as Arc<dyn Module>);
        registry.register(Arc::clone(&c_named) as Arc<dyn Module>);

        let bootstrap = Bootstrap::new(registry);
        let ctx = make_ctx();

        let result = bootstrap.start_all(ctx).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            BootstrapError::StartFailed { module, .. } => {
                assert_eq!(module, "c");
            }
            other => panic!("expected StartFailed, got: {other}"),
        }

        // A and B should have been started and then stopped during rollback
        assert!(a_named.is_started());
        assert!(b_named.is_started());
        assert!(!c_named.is_started()); // C never started (failed)
    }

    /// Stop failure does not prevent subsequent modules from being stopped.
    #[tokio::test]
    async fn test_stop_failure_continues() {
        let mut registry = ModuleRegistry::new();
        let a_named = Arc::new(NamedModule::new("a", &[]));
        let b_named = Arc::new(NamedModule::new("b", &["a"]));
        let c_named = Arc::new(NamedModule::new("c", &["b"]));
        registry.register(Arc::clone(&a_named) as Arc<dyn Module>);
        registry.register(Arc::clone(&b_named) as Arc<dyn Module>);
        registry.register(Arc::clone(&c_named) as Arc<dyn Module>);

        let bootstrap = Bootstrap::new(registry);
        let ctx = make_ctx();

        // Start all first
        bootstrap.start_all(ctx.clone()).await.unwrap();
        assert!(a_named.is_started());
        assert!(b_named.is_started());
        assert!(c_named.is_started());

        // Now mark B to fail on stop
        b_named
            .fail_on_stop
            .store(true, std::sync::atomic::Ordering::SeqCst);

        // stop_all should complete without panicking
        bootstrap.stop_all().await;

        // C and A should have been stopped (shutdown order: C, B, A)
        assert!(c_named.is_stopped());
        assert!(a_named.is_stopped());
        // B attempted stop but failed — still not stopped
        assert!(!b_named.is_stopped());
    }

    /// Full integration: register, start, stop lifecycle.
    #[tokio::test]
    async fn test_full_start_stop_lifecycle() {
        let mut registry = ModuleRegistry::new();
        let a_named = Arc::new(NamedModule::new("a", &[]));
        let b_named = Arc::new(NamedModule::new("b", &["a"]));
        let c_named = Arc::new(NamedModule::new("c", &["b"]));
        registry.register(Arc::clone(&a_named) as Arc<dyn Module>);
        registry.register(Arc::clone(&b_named) as Arc<dyn Module>);
        registry.register(Arc::clone(&c_named) as Arc<dyn Module>);

        let bootstrap = Bootstrap::new(registry);
        let ctx = make_ctx();

        // Start all
        bootstrap.start_all(ctx).await.unwrap();
        assert!(a_named.is_started());
        assert!(b_named.is_started());
        assert!(c_named.is_started());

        // Stop all
        bootstrap.stop_all().await;
        assert!(a_named.is_stopped());
        assert!(b_named.is_stopped());
        assert!(c_named.is_stopped());
    }

    /// get() retrieves modules by name.
    #[test]
    fn test_registry_get() {
        let mut registry = ModuleRegistry::new();
        let a = make_module("alpha", &[]);
        let b = make_module("beta", &[]);
        registry.register(Arc::clone(&a));
        registry.register(Arc::clone(&b));

        assert!(registry.get("alpha").is_some());
        assert_eq!(registry.get("alpha").unwrap().name(), "alpha");
        assert!(registry.get("gamma").is_none());
    }

    /// all() returns all modules in registration order.
    #[test]
    fn test_registry_all() {
        let mut registry = ModuleRegistry::new();
        let a = make_module("a", &[]);
        let b = make_module("b", &[]);
        registry.register(Arc::clone(&a));
        registry.register(Arc::clone(&b));

        let all = registry.all();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name(), "a");
        assert_eq!(all[1].name(), "b");
    }
}
