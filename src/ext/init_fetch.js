import * as fetch from "ext:deno_fetch/26_fetch.js";

Deno.core.setWasmStreamingCallback(fetch.handleWasmStreaming);

import { applyToGlobal, writeable } from 'ext:rustyscript/rustyscript.js';
applyToGlobal({
    fetch: writeable(fetch.fetch),
});