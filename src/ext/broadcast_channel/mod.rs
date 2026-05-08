use deno_core::{extension, Extension};
use deno_web::InMemoryBroadcastChannel;

use super::ExtensionTrait;

mod wrapper;
pub use wrapper::{
    BroadcastChannelWrapper, IsolatedBroadcastChannel, IsolatedBroadcastChannelWrapper,
};

extension!(
    init_broadcast_channel,
    deps = [rustyscript],
    esm_entry_point = "ext:init_broadcast_channel/init_broadcast_channel.js",
    esm = [ dir "src/ext/broadcast_channel", "init_broadcast_channel.js" ],
);

extension!(
    deno_broadcast_channel,
    deps = [deno_web],
    esm = [ dir "src/ext/broadcast_channel", "01_broadcast_channel.js" ],
);

impl ExtensionTrait<()> for init_broadcast_channel {
    fn init((): ()) -> Extension {
        init_broadcast_channel::init()
    }
}

impl ExtensionTrait<()> for deno_broadcast_channel {
    fn init((): ()) -> Extension {
        deno_broadcast_channel::init()
    }
}

// Note: broadcast_channel functionality is now integrated into deno_web
// No separate initialization is needed as it's handled by deno_web extension
pub fn extensions(_channel: InMemoryBroadcastChannel, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_broadcast_channel::build((), is_snapshot),
        init_broadcast_channel::build((), is_snapshot),
    ]
}

#[cfg(test)]
mod test {
    use deno_core::PollEventLoopOptions;

    use crate::{module, BroadcastChannelWrapper, Module, Runtime, RuntimeOptions};

    static TEST_MOD: Module = module!(
        "test.js",
        "
        const channel = new BroadcastChannel('my_channel');
        channel.onmessage = (event) => {
            channel.postMessage('Received: ' + event.data);
        };
    "
    );

    /// This test is identical to the original test from origin/master that verified
    /// JavaScript ↔ Rust bidirectional communication.
    ///
    /// `BroadcastChannelWrapper` has been restored to maintain full backward compatibility
    /// with origin/master behavior.
    #[test]
    fn test_broadcast_channel() {
        let options = RuntimeOptions::default();
        let channel = options.extension_options.broadcast_channel.clone();

        let mut runtime = Runtime::new(options).unwrap();
        let tokio_runtime = runtime.tokio_runtime();

        let channel = BroadcastChannelWrapper::new(&channel, "my_channel").unwrap();

        tokio_runtime
            .block_on(runtime.load_module_async(&TEST_MOD))
            .unwrap();

        channel.send_sync(&mut runtime, "foo").unwrap();

        runtime
            .block_on_event_loop(
                PollEventLoopOptions::default(),
                Some(std::time::Duration::from_secs(1)),
            )
            .unwrap();

        let value = channel
            .recv_sync::<String>(&mut runtime, Some(std::time::Duration::from_secs(1)))
            .unwrap()
            .unwrap();

        assert_eq!(value, "Received: foo");
    }
}
