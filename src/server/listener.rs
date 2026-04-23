//! RegistryListener — watches the apcore registry for changes and
//! synchronizes the MCP tool list.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use apcore::registry::{ModuleDescriptor, Registry};

use crate::server::factory::MCPServerFactory;
use crate::server::types::Tool;

/// Listens for registry events and keeps the MCP tool list in sync.
///
/// The listener maintains a thread-safe `HashMap<String, Tool>` that is
/// updated when modules are registered or unregistered in the apcore registry.
pub struct RegistryListener {
    tools: Arc<RwLock<HashMap<String, Tool>>>,
    active: Arc<AtomicBool>,
}

impl RegistryListener {
    /// Create a new registry listener.
    ///
    /// The listener starts in an inactive state. Call [`start`](Self::start)
    /// to begin processing events.
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start listening for registry change events.
    ///
    /// Registers callbacks on the registry for "register" and "unregister" events.
    /// Idempotent: calling multiple times is safe (second call is a no-op).
    pub fn start(&self, registry: &mut Registry, factory: Arc<MCPServerFactory>) {
        // compare_exchange ensures only one activation succeeds
        if self
            .active
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return; // already active
        }

        // Register "register" callback
        {
            let tools = Arc::clone(&self.tools);
            let active = Arc::clone(&self.active);
            let factory = Arc::clone(&factory);
            registry.on(
                "register",
                Box::new(
                    move |module_id: &str, _module: &dyn apcore::module::Module| {
                        if !active.load(Ordering::SeqCst) {
                            return;
                        }
                        // Build a tool from the module's own schema info.
                        // The callback receives the module reference directly, so we
                        // construct a descriptor from it.
                        let description = _module.description().to_string();
                        let input_schema = _module.input_schema();
                        let output_schema = _module.output_schema();

                        let descriptor = ModuleDescriptor {
                            module_id: module_id.to_string(),
                            name: None,
                            description: description.clone(),
                            documentation: None,
                            input_schema,
                            output_schema,
                            version: "1.0.0".to_string(),
                            tags: vec![],
                            annotations: Some(apcore::module::ModuleAnnotations::default()),
                            examples: vec![],
                            metadata: std::collections::HashMap::new(),
                            display: None,
                            sunset_date: None,
                            dependencies: vec![],
                            enabled: true,
                        };

                        match factory.build_tool(&descriptor, &description, None) {
                            Ok(tool) => {
                                if let Ok(mut map) = tools.write() {
                                    map.insert(module_id.to_string(), tool);
                                }
                                tracing::info!("Tool registered: {}", module_id);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to build tool for {}: {}", module_id, e);
                            }
                        }
                    },
                ),
            );
        }

        // Register "unregister" callback
        {
            let tools = Arc::clone(&self.tools);
            let active = Arc::clone(&self.active);
            registry.on(
                "unregister",
                Box::new(
                    move |module_id: &str, _module: &dyn apcore::module::Module| {
                        if !active.load(Ordering::SeqCst) {
                            return;
                        }
                        let removed = if let Ok(mut map) = tools.write() {
                            map.remove(module_id).is_some()
                        } else {
                            false
                        };
                        if removed {
                            tracing::info!("Tool unregistered: {}", module_id);
                        }
                    },
                ),
            );
        }
    }

    /// Stop listening for registry change events.
    ///
    /// Sets the internal active flag to `false`, causing subsequent callback
    /// invocations to no-op. The apcore Registry does not support callback
    /// removal, so the callbacks remain registered but inactive.
    pub fn stop(&self) {
        self.active.store(false, Ordering::SeqCst);
    }

    /// Return a snapshot of currently registered tools. Thread-safe.
    pub fn tools(&self) -> HashMap<String, Tool> {
        self.tools
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Return whether the listener is currently active.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Directly insert a tool (for testing or manual registration).
    #[cfg(test)]
    fn insert_tool(&self, module_id: &str, tool: Tool) {
        if let Ok(mut map) = self.tools.write() {
            map.insert(module_id.to_string(), tool);
        }
    }

    /// Directly remove a tool (for testing or manual unregistration).
    #[cfg(test)]
    fn remove_tool(&self, module_id: &str) -> bool {
        if let Ok(mut map) = self.tools.write() {
            map.remove(module_id).is_some()
        } else {
            false
        }
    }
}

impl Default for RegistryListener {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_test_tool(name: &str) -> Tool {
        Tool {
            name: name.to_string(),
            description: format!("Test tool {}", name),
            input_schema: json!({"type": "object", "properties": {}}),
            annotations: None,
            meta: None,
        }
    }

    // --- Helper: DummyModule that implements the Module trait correctly ---

    macro_rules! define_dummy_module {
        ($name:ident, $desc:expr) => {
            #[derive(Debug)]
            struct $name;

            #[async_trait::async_trait]
            impl apcore::module::Module for $name {
                fn input_schema(&self) -> serde_json::Value {
                    json!({"type": "object", "properties": {}})
                }
                fn output_schema(&self) -> serde_json::Value {
                    json!({"type": "object"})
                }
                fn description(&self) -> &str {
                    $desc
                }
                async fn execute(
                    &self,
                    _inputs: serde_json::Value,
                    _ctx: &apcore::context::Context<serde_json::Value>,
                ) -> Result<serde_json::Value, apcore::errors::ModuleError> {
                    Ok(json!({}))
                }
            }
        };
    }

    // ---- Unit tests for tools map ----

    #[test]
    fn tools_returns_empty_map_on_fresh_listener() {
        let listener = RegistryListener::new();
        assert!(listener.tools().is_empty());
    }

    #[test]
    fn tools_returns_snapshot_not_reference() {
        let listener = RegistryListener::new();
        listener.insert_tool("mod_a", make_test_tool("mod_a"));

        let snapshot1 = listener.tools();
        let snapshot2 = listener.tools();

        assert_eq!(snapshot1.len(), 1);
        assert_eq!(snapshot2.len(), 1);
        assert!(snapshot1.contains_key("mod_a"));
        assert!(snapshot2.contains_key("mod_a"));
    }

    #[test]
    fn snapshot_is_isolated_from_later_mutations() {
        let listener = RegistryListener::new();
        listener.insert_tool("mod_a", make_test_tool("mod_a"));

        let snapshot = listener.tools();
        assert_eq!(snapshot.len(), 1);

        // Mutate after taking snapshot
        listener.insert_tool("mod_b", make_test_tool("mod_b"));
        // Original snapshot must not contain mod_b
        assert_eq!(snapshot.len(), 1);
        assert!(!snapshot.contains_key("mod_b"));
        // But a fresh snapshot does
        assert_eq!(listener.tools().len(), 2);
    }

    #[test]
    fn default_creates_inactive_listener() {
        let listener = RegistryListener::default();
        assert!(!listener.is_active());
        assert!(listener.tools().is_empty());
    }

    #[test]
    fn concurrent_register_unregister_stress() {
        use std::thread;

        let listener = Arc::new(RegistryListener::new());
        let mut handles = vec![];

        // Spawn 20 threads that register tools
        for i in 0..20 {
            let listener = Arc::clone(&listener);
            handles.push(thread::spawn(move || {
                let name = format!("stress_{}", i);
                listener.insert_tool(&name, make_test_tool(&name));
            }));
        }

        // Spawn 10 threads that unregister tools (some may not exist yet)
        for i in 0..10 {
            let listener = Arc::clone(&listener);
            handles.push(thread::spawn(move || {
                let name = format!("stress_{}", i);
                listener.remove_tool(&name);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // Exact count depends on timing, but should be between 10 and 20
        let count = listener.tools().len();
        assert!(
            (10..=20).contains(&count),
            "unexpected tool count: {}",
            count
        );
    }

    #[test]
    fn after_register_tools_contains_tool() {
        let listener = RegistryListener::new();
        listener.insert_tool("mod_a", make_test_tool("mod_a"));

        let tools = listener.tools();
        assert_eq!(tools.len(), 1);
        assert!(tools.contains_key("mod_a"));
        assert_eq!(tools["mod_a"].name, "mod_a");
    }

    #[test]
    fn after_unregister_tools_no_longer_contains_tool() {
        let listener = RegistryListener::new();
        listener.insert_tool("mod_a", make_test_tool("mod_a"));
        assert_eq!(listener.tools().len(), 1);

        let removed = listener.remove_tool("mod_a");
        assert!(removed);
        assert!(listener.tools().is_empty());
    }

    #[test]
    fn unregister_nonexistent_tool_returns_false() {
        let listener = RegistryListener::new();
        let removed = listener.remove_tool("nonexistent");
        assert!(!removed);
    }

    #[test]
    fn multiple_tools_registered_and_unregistered() {
        let listener = RegistryListener::new();
        listener.insert_tool("mod_a", make_test_tool("mod_a"));
        listener.insert_tool("mod_b", make_test_tool("mod_b"));
        listener.insert_tool("mod_c", make_test_tool("mod_c"));

        assert_eq!(listener.tools().len(), 3);

        listener.remove_tool("mod_b");
        let tools = listener.tools();
        assert_eq!(tools.len(), 2);
        assert!(tools.contains_key("mod_a"));
        assert!(!tools.contains_key("mod_b"));
        assert!(tools.contains_key("mod_c"));
    }

    // ---- Start / stop tests ----

    #[test]
    fn start_is_idempotent() {
        let listener = RegistryListener::new();
        let factory = Arc::new(MCPServerFactory::new());
        let mut registry = Registry::new();

        listener.start(&mut registry, Arc::clone(&factory));
        assert!(listener.is_active());

        // Second call should be a no-op (no panic, still active)
        listener.start(&mut registry, factory);
        assert!(listener.is_active());
    }

    #[test]
    fn stop_deactivates_listener() {
        let listener = RegistryListener::new();
        let factory = Arc::new(MCPServerFactory::new());
        let mut registry = Registry::new();

        listener.start(&mut registry, factory);
        assert!(listener.is_active());

        listener.stop();
        assert!(!listener.is_active());
    }

    #[test]
    fn stop_is_idempotent() {
        let listener = RegistryListener::new();
        listener.stop(); // no-op when not started
        assert!(!listener.is_active());

        let factory = Arc::new(MCPServerFactory::new());
        let mut registry = Registry::new();
        listener.start(&mut registry, factory);
        listener.stop();
        listener.stop(); // second stop is no-op
        assert!(!listener.is_active());
    }

    // ---- Thread-safety test ----

    #[test]
    fn listener_is_thread_safe() {
        use std::thread;

        let listener = Arc::new(RegistryListener::new());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let listener = Arc::clone(&listener);
                thread::spawn(move || {
                    let name = format!("mod_{}", i);
                    listener.insert_tool(&name, make_test_tool(&name));
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(listener.tools().len(), 10);
    }

    // ---- Integration tests with real Registry ----

    define_dummy_module!(DummyModuleA, "A dummy module A");
    define_dummy_module!(DummyModuleB, "A dummy module B");
    define_dummy_module!(DummyModuleC, "A dummy module C");

    #[test]
    fn start_registers_callbacks_that_respond_to_events() {
        let listener = RegistryListener::new();
        let factory = Arc::new(MCPServerFactory::new());
        let mut registry = Registry::new();

        listener.start(&mut registry, factory);

        let descriptor = apcore::registry::ModuleDescriptor {
            module_id: "dummy_a".to_string(),
            name: None,
            description: String::new(),
            documentation: None,
            input_schema: json!({"type": "object", "properties": {}}),
            output_schema: json!({"type": "object"}),
            version: "1.0.0".to_string(),
            tags: vec![],
            annotations: Some(apcore::module::ModuleAnnotations::default()),
            examples: vec![],
            metadata: std::collections::HashMap::new(),
            display: None,
            sunset_date: None,
            dependencies: vec![],
            enabled: true,
        };

        registry
            .register("dummy_a", Box::new(DummyModuleA), descriptor)
            .unwrap();

        let tools = listener.tools();
        assert_eq!(tools.len(), 1);
        assert!(tools.contains_key("dummy_a"));
    }

    #[test]
    fn stopped_listener_ignores_register_events() {
        let listener = RegistryListener::new();
        let factory = Arc::new(MCPServerFactory::new());
        let mut registry = Registry::new();

        listener.start(&mut registry, Arc::clone(&factory));
        listener.stop();

        let descriptor = apcore::registry::ModuleDescriptor {
            module_id: "dummy_b".to_string(),
            name: None,
            description: String::new(),
            documentation: None,
            input_schema: json!({"type": "object", "properties": {}}),
            output_schema: json!({"type": "object"}),
            version: "1.0.0".to_string(),
            tags: vec![],
            annotations: Some(apcore::module::ModuleAnnotations::default()),
            examples: vec![],
            metadata: std::collections::HashMap::new(),
            display: None,
            sunset_date: None,
            dependencies: vec![],
            enabled: true,
        };

        registry
            .register("dummy_b", Box::new(DummyModuleB), descriptor)
            .unwrap();

        // Listener is stopped, so tools should remain empty
        assert!(listener.tools().is_empty());
    }

    #[test]
    fn unregister_event_removes_tool() {
        let listener = RegistryListener::new();
        let factory = Arc::new(MCPServerFactory::new());
        let mut registry = Registry::new();

        listener.start(&mut registry, Arc::clone(&factory));

        let descriptor = apcore::registry::ModuleDescriptor {
            module_id: "dummy_c".to_string(),
            name: None,
            description: String::new(),
            documentation: None,
            input_schema: json!({"type": "object", "properties": {}}),
            output_schema: json!({"type": "object"}),
            version: "1.0.0".to_string(),
            tags: vec![],
            annotations: Some(apcore::module::ModuleAnnotations::default()),
            examples: vec![],
            metadata: std::collections::HashMap::new(),
            display: None,
            sunset_date: None,
            dependencies: vec![],
            enabled: true,
        };

        registry
            .register("dummy_c", Box::new(DummyModuleC), descriptor)
            .unwrap();
        assert_eq!(listener.tools().len(), 1);

        registry.unregister("dummy_c").unwrap();
        assert!(listener.tools().is_empty());
    }
}
