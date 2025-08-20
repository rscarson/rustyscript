use deno_core::extension;

use crate::ext::ExtensionList;

mod wrapper;
pub use wrapper::BroadcastChannelWrapper;

extension!(
    broadcast_channel,
    deps = [rustyscript],
    esm_entry_point = "ext:broadcast_channel/init_broadcast_channel.js",
    esm = [ dir "src/ext/broadcast_channel", "init_broadcast_channel.js" ],
);

pub fn load(extensions: &mut ExtensionList) {
    let options = extensions.options();
    extensions.extend([
        deno_broadcast_channel::deno_broadcast_channel::init(options.broadcast_channel.clone()),
        broadcast_channel::init(),
    ]);
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
        let mut runtime = Runtime::new(RuntimeOptions::default()).unwrap();
        let tokio_runtime = runtime.tokio_runtime();

        let channel = runtime.extension_options().broadcast_channel.clone();
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
