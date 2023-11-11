use deno_core::{extension, Extension};
extension!(
    deno_web,
    deps = [rustyscript],
    esm_entry_point = "ext:deno_web/01_dom_exception.js",
    esm = [ dir "src/ext/web_stub", "01_dom_exception.js" ],
);

pub fn extensions() -> Vec<Extension> {
    vec![deno_web::init_ops_and_esm()]
}
