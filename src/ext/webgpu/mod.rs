use deno_core::extension;

use crate::ext::ExtensionList;

extension!(
    webgpu,
    deps = [rustyscript],
    esm_entry_point = "ext:webgpu/init_webgpu.js",
    esm = [ dir "src/ext/webgpu", "init_webgpu.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    extensions.extend([deno_webgpu::deno_webgpu::init(), webgpu::init()]);
}
