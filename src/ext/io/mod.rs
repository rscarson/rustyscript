use deno_core::extension;

use crate::ext::ExtensionList;

mod tty;

extension!(
    io,
    deps = [rustyscript],
    esm_entry_point = "ext:io/init_io.js",
    esm = [ dir "src/ext/io", "init_io.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    extensions.extend([
        deno_io::deno_io::init(options.io_pipes.clone()),
        tty::deno_tty::init(),
        io::init(),
    ]);
}
