use deno_core::{extension, Extension};

use super::ExtensionTrait;

extension!(
    init_webgpu,
    deps = [rustyscript],
    esm_entry_point = "ext:init_webgpu/init_webgpu.js",
    esm = [ dir "src/ext/webgpu", "init_webgpu.js" ],
);
impl ExtensionTrait<()> for init_webgpu {
    fn init((): ()) -> Extension {
        init_webgpu::init()
    }
}
impl ExtensionTrait<()> for deno_webgpu::deno_webgpu {
    fn init((): ()) -> Extension {
        deno_webgpu::deno_webgpu::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_webgpu::deno_webgpu::build((), is_snapshot),
        init_webgpu::build((), is_snapshot),
    ]
}
