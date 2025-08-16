use deno_core::{v8, ModuleId};

use crate::Module;

/// Represents a loaded instance of a module within a runtime
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ModuleHandle {
    entrypoint: Option<v8::Global<v8::Function>>,
    module_id: ModuleId,
    module: Module,
}

impl ModuleHandle {
    /// Create a new module instance
    pub(crate) fn new(
        module: &Module,
        module_id: ModuleId,
        entrypoint: Option<v8::Global<v8::Function>>,
    ) -> Self {
        Self {
            module_id,
            entrypoint,
            module: module.clone(),
        }
    }

    /// Create a new module handle from raw parts
    ///
    /// # Safety
    /// This function is unsafe because it allows using potentially invalid `ModuleIds`.
    ///
    /// Use of an unloaded module ID will result in a panic.
    #[must_use]
    pub unsafe fn from_raw(
        module: &Module,
        module_id: ModuleId,
        entrypoint: Option<v8::Global<v8::Function>>,
    ) -> Self {
        Self::new(module, module_id, entrypoint)
    }

    /// Return this module's contents
    #[must_use]
    pub fn module(&self) -> &Module {
        &self.module
    }

    /// Return this module's ID
    #[must_use]
    pub fn id(&self) -> ModuleId {
        self.module_id
    }

    /// Return this module's entrypoint
    #[must_use]
    pub fn entrypoint(&self) -> &Option<v8::Global<v8::Function>> {
        &self.entrypoint
    }
}
