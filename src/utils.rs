//! Internal utilities — registry and executor resolution.
//!
//! These are not part of the public API.

#![allow(unused)]

use std::sync::Arc;

use apcore::approval::ApprovalHandler;
use apcore::config::Config;
use apcore::executor::Executor;
use apcore::registry::registry::Registry;

use crate::apcore_mcp::{APCoreMCPError, BackendSource};

/// Resolve a [`Registry`] from the given [`BackendSource`].
///
/// - `ExtensionsDir`: not yet supported (requires discover integration).
/// - `Registry`: returns the `Arc<Registry>` directly.
/// - `Executor`: extracts the registry from the executor (cloned into a new Arc).
pub fn resolve_registry(source: &BackendSource) -> Result<Arc<Registry>, APCoreMCPError> {
    match source {
        BackendSource::ExtensionsDir(path) => {
            // TODO: Create Registry with discoverer pointed at `path`, call discover().
            // For now, return an error since discover() integration is not yet wired.
            Err(APCoreMCPError::BackendResolution(format!(
                "ExtensionsDir resolution not yet implemented for path: {}",
                path.display()
            )))
        }
        BackendSource::Registry(reg) => Ok(Arc::clone(reg)),
        BackendSource::Executor(exec) => {
            // The Executor holds a Registry by value. We need to return an Arc<Registry>.
            // Since we cannot move it out, we return an error directing users to provide
            // a Registry directly, or we clone if Registry implements Clone.
            // Registry does not implement Clone, so we return a BackendResolution error
            // explaining the limitation.
            Err(APCoreMCPError::BackendResolution(
                "cannot extract registry from Executor: provide a Registry or ExtensionsDir source instead".to_string()
            ))
        }
    }
}

/// Resolve an [`Executor`] from the given [`BackendSource`].
///
/// - `ExtensionsDir`: not yet supported (requires discover integration).
/// - `Registry`: creates a new `Executor` from the registry (cloned).
/// - `Executor`: returns the `Arc<Executor>` directly.
pub fn resolve_executor(
    source: &BackendSource,
    approval_handler: Option<Box<dyn ApprovalHandler>>,
) -> Result<Arc<Executor>, APCoreMCPError> {
    match source {
        BackendSource::ExtensionsDir(path) => {
            // TODO: Create Registry, discover, then build Executor.
            Err(APCoreMCPError::BackendResolution(format!(
                "ExtensionsDir resolution not yet implemented for path: {}",
                path.display()
            )))
        }
        BackendSource::Registry(_reg) => {
            // Registry does not implement Clone, so we cannot move it out of Arc.
            // Once we have discover() integration, this will create a fresh executor.
            Err(APCoreMCPError::BackendResolution(
                "cannot create Executor from Arc<Registry>: provide an Executor or ExtensionsDir source instead".to_string()
            ))
        }
        BackendSource::Executor(exec) => Ok(Arc::clone(exec)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn resolve_registry_from_registry_returns_same() {
        let reg = Arc::new(Registry::new());
        let source = BackendSource::Registry(reg.clone());
        let resolved = resolve_registry(&source).unwrap();
        assert!(Arc::ptr_eq(&reg, &resolved));
    }

    #[test]
    fn resolve_executor_from_executor_returns_same() {
        let reg = Registry::new();
        let exec = Arc::new(Executor::new(reg, Config::default()));
        let source = BackendSource::Executor(exec.clone());
        let resolved = resolve_executor(&source, None).unwrap();
        assert!(Arc::ptr_eq(&exec, &resolved));
    }

    #[test]
    fn resolve_registry_from_extensions_dir_returns_error() {
        let source = BackendSource::ExtensionsDir(PathBuf::from("./test-ext"));
        let result = resolve_registry(&source);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not yet implemented"));
    }

    #[test]
    fn resolve_executor_from_extensions_dir_returns_error() {
        let source = BackendSource::ExtensionsDir(PathBuf::from("./test-ext"));
        let result = resolve_executor(&source, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not yet implemented"));
    }

    #[test]
    fn resolve_registry_from_executor_returns_error() {
        let reg = Registry::new();
        let exec = Arc::new(Executor::new(reg, Config::default()));
        let source = BackendSource::Executor(exec);
        let result = resolve_registry(&source);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot extract"));
    }

    #[test]
    fn resolve_executor_from_registry_returns_error() {
        let reg = Arc::new(Registry::new());
        let source = BackendSource::Registry(reg);
        let result = resolve_executor(&source, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot create"));
    }
}
