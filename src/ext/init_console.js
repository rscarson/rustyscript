import * as console from 'ext:deno_console/01_console.js';

import { applyToGlobal, nonEnumerable } from 'ext:js_playground/js_playground.js';
applyToGlobal({
    console: nonEnumerable(
      new console.Console((msg, level) => globalThis.Deno.core.print(msg, level > 1)),
    ),
});