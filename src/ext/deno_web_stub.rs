use deno_core::extension;

extension!(
    deno_web,
    esm_entry_point = "ext:deno_web/01_dom_exception.js",
    esm = [ dir "src/ext", "01_dom_exception.js" ],
);
