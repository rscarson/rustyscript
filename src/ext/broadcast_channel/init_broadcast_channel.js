import * as broadcastChannel from "ext:deno_broadcast_channel/01_broadcast_channel.js";

import { applyToGlobal, nonEnumerable } from 'ext:rustyscript/rustyscript.js';
applyToGlobal({
    BroadcastChannel: nonEnumerable(broadcastChannel.BroadcastChannel),
});