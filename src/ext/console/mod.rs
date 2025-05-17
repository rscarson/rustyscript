use super::ExtensionTrait;
use deno_core::{extension, Extension};

extension!(
    init_console,
    deps = [rustyscript],
    esm_entry_point = "ext:init_console/init_console.js",
    esm = [ dir "src/ext/console", "init_console.js" ],
);
impl ExtensionTrait<()> for init_console {
    fn init((): ()) -> Extension {
        deno_terminal::colors::set_use_color(true);
        init_console::init()
    }
}
impl ExtensionTrait<()> for deno_console::deno_console {
    fn init((): ()) -> Extension {
        deno_console::deno_console::init()
    }
}

pub fn extensions(is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_console::deno_console::build((), is_snapshot),
        init_console::build((), is_snapshot),
    ]
}
