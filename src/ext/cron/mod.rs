use deno_core::extension;
use deno_cron::local::LocalCronHandler;

use crate::ext::ExtensionList;

extension!(
    cron,
    deps = [rustyscript],
    esm_entry_point = "ext:cron/init_cron.js",
    esm = [ dir "src/ext/cron", "init_cron.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    extensions.extend([
        deno_cron::deno_cron::init(LocalCronHandler::new()),
        cron::init(),
    ]);
}
