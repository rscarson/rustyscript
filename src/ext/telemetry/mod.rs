use super::ExtensionTrait;
use deno_core::{extension, Extension};

extension!(
    init_telemetry,
    deps = [rustyscript],
    esm_entry_point = "ext:init_telemetry/init_telemetry.js",
    esm = [ dir "src/ext/telemetry", "init_telemetry.js" ],
);
impl ExtensionTrait<()> for init_telemetry {
    fn init((): ()) -> Extension {
        init_telemetry::init_ops_and_esm()
    }
}

impl ExtensionTrait<()> for deno_telemetry::deno_telemetry {
    fn init((): ()) -> Extension {
        deno_telemetry::deno_telemetry::init_ops_and_esm()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_telemetry::deno_telemetry::build((), is_snapshot),
        init_telemetry::build((), is_snapshot),
    ]
}
