use deno_core::extension;

use crate::ext::ExtensionList;

extension!(
    console,
    deps = [rustyscript],
    esm_entry_point = "ext:console/init_console.js",
    esm = [ dir "src/ext/console", "init_console.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    deno_terminal::colors::set_use_color(true);
    extensions.extend([deno_console::deno_console::init(), console::init()]);
}
