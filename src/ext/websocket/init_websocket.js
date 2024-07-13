import * as websocket from "ext:deno_websocket/01_websocket.js";
import * as websocketStream from "ext:deno_websocket/02_websocketstream.js";

import { applyToGlobal, getterOnly, nonEnumerable } from 'ext:rustyscript/rustyscript.js';

applyToGlobal({
    WebSocket: nonEnumerable(websocket.WebSocket),
    CloseEvent: nonEnumerable(websocket.CloseEvent)
});