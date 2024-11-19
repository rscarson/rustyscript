import * as broadcastChannel from "ext:deno_broadcast_channel/01_broadcast_channel.js";
import { core } from "ext:core/mod.js";

import { applyToGlobal, nonEnumerable } from 'ext:rustyscript/rustyscript.js';
applyToGlobal({
    BroadcastChannel: nonEnumerable(broadcastChannel.BroadcastChannel),
    serialize: nonEnumerable(core.serialize),
    deserialize: nonEnumerable(core.deserialize),
});
