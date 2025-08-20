use deno_core::extension;

use crate::ext::ExtensionList;

extension!(
    crypto,
    deps = [rustyscript],
    esm_entry_point = "ext:crypto/init_crypto.js",
    esm = [ dir "src/ext/crypto", "init_crypto.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    extensions.extend([
        deno_crypto::deno_crypto::init(options.crypto_seed),
        crypto::init(),
    ]);
}
