use super::ExtensionTrait;
use crate::{big_json_args, Error, Runtime};
use deno_broadcast_channel::{BroadcastChannel, InMemoryBroadcastChannel};
use deno_core::{extension, Extension};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

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
        init_broadcast_channel::init_ops_and_esm()
    }
}
impl ExtensionTrait<InMemoryBroadcastChannel> for deno_broadcast_channel::deno_broadcast_channel {
    fn init(channel: InMemoryBroadcastChannel) -> Extension {
        deno_broadcast_channel::deno_broadcast_channel::init_ops_and_esm(channel)
    }
}

pub fn extensions(channel: InMemoryBroadcastChannel, is_snapshot: bool) -> Vec<Extension> {
    vec![
        deno_broadcast_channel::deno_broadcast_channel::build(channel, is_snapshot),
        init_broadcast_channel::build((), is_snapshot),
    ]
}
