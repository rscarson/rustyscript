import * as headers from "ext:deno_fetch/20_headers.js";
import * as formData from "ext:deno_fetch/21_formdata.js";
import * as request from "ext:deno_fetch/23_request.js";
import * as response from "ext:deno_fetch/23_response.js";
import * as fetch from "ext:deno_fetch/26_fetch.js";

Deno.core.setWasmStreamingCallback(fetch.handleWasmStreaming);

import { applyToGlobal, writeable } from 'ext:js_playground/js_playground.js';
applyToGlobal({
    fetch: writeable(fetch.fetch),
});