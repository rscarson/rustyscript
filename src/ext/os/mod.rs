use super::ExtensionTrait;
use deno_core::{extension, op2, Extension};

/// Exit the process with the given exit code
#[op2(fast)]
fn op_rustyscript_exit(#[smi] code: i32) {
    std::process::exit(code);
}

extension!(
    init_os,
    deps = [rustyscript],
    ops = [op_rustyscript_exit],
    esm_entry_point = "ext:init_os/init_os.js",
    esm = [ dir "src/ext/os", "init_os.js" ],
);

impl ExtensionTrait<()> for init_os {
    fn init((): ()) -> Extension {
        init_os::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![init_os::build((), is_snapshot)]
}

#[cfg(test)]
mod test;