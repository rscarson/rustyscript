import * as DOMException from 'ext:deno_web/01_dom_exception.js';
import * as timers from 'ext:deno_web/02_timers.js';

import { applyToGlobal, nonEnumerable, writeable } from 'ext:rustyscript/rustyscript.js';
applyToGlobal({
    DOMException: nonEnumerable(DOMException),

    setImmediate: writeable(timers.setImmediate),
    clearInterval: writeable(timers.clearInterval),
    clearTimeout: writeable(timers.clearTimeout),
    setInterval: writeable(timers.setInterval),
    setTimeout: writeable(timers.setTimeout),
});