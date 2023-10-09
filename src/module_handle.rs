use deno_core::v8;
use deno_core::ModuleId;

use crate::Module;

/// Represents a loaded instance of a module within a runtime
#[derive(Debug, Default)]
pub struct ModuleHandle {
    entrypoint: Option<v8::Global<v8::Function>>,
    module_id: ModuleId,
    module: Module,
}

impl ModuleHandle {
    /// Create a new module instance
    pub fn new(
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

    /// Return this module's contents
    pub fn module(&self) -> &Module {
        &self.module
    }

    /// Return this module's ID
    pub fn id(&self) -> ModuleId {
        self.module_id
    }

    /// Return this module's entrypoint
    pub fn entrypoint(&self) -> &Option<v8::Global<v8::Function>> {
        &self.entrypoint
    }
}
