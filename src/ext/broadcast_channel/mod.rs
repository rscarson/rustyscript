use deno_broadcast_channel::InMemoryBroadcastChannel;
use deno_core::{extension, Extension};

use super::ExtensionTrait;

mod wrapper;
pub use wrapper::BroadcastChannelWrapper;

extension!(
    init_broadcast_channel,
    deps = [rustyscript],
    esm_entry_point = "ext:init_broadcast_channel/init_broadcast_channel.js",
    esm = [ dir "src/ext/broadcast_channel", "init_broadcast_channel.js" ],
);
impl ExtensionTrait<()> for init_broadcast_channel {
    fn init((): ()) -> Extension {
        init_broadcast_channel::init()
    }
}
impl ExtensionTrait<InMemoryBroadcastChannel> for deno_broadcast_channel::deno_broadcast_channel {
    fn init(channel: InMemoryBroadcastChannel) -> Extension {
        deno_broadcast_channel::deno_broadcast_channel::init(channel)
    }
}

pub fn extensions(channel: InMemoryBroadcastChannel, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_broadcast_channel::deno_broadcast_channel::build(channel, is_snapshot),
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
