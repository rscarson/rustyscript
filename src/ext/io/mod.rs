use deno_core::{extension, Extension};

#[cfg(windows)]
mod tty_windows;
#[cfg(windows)]
use tty_windows as tty;

#[cfg(unix)]
mod tty_unix;
#[cfg(unix)]
use tty_unix as tty;

extension!(
    init_io,
    deps = [rustyscript],
    esm_entry_point = "ext:init_io/init_io.js",
    esm = [ dir "src/ext/io", "init_io.js" ],
);

pub fn extensions(pipes: Option<deno_io::Stdio>) -> Vec<Extension> {
    vec![
        tty::deno_tty::init_ops_and_esm(),
        init_io::init_ops_and_esm(),
        deno_io::deno_io::init_ops_and_esm(pipes),
    ]
}

pub fn snapshot_extensions(pipes: Option<deno_io::Stdio>) -> Vec<Extension> {
    vec![
        tty::deno_tty::init_ops(),
        init_io::init_ops(),
        deno_io::deno_io::init_ops(pipes),
    ]
}
