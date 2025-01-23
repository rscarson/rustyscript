import * as _console from 'ext:deno_console/01_console.js';

import { applyToGlobal, nonEnumerable } from 'ext:rustyscript/rustyscript.js';
applyToGlobal({
    console: nonEnumerable(
      new _console.Console((msg, _level) => {
          try {
              rustyscript.functions['console.log'](msg);
          } catch (_e) {
              // ignore
              // NOTE(ysh)
              // This can happen while console.log is not registered
          }
      }),
    ),
});

globalThis.Deno.inspect = _console.inspect;