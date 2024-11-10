import * as _console from 'ext:deno_console/01_console.js';

import { applyToGlobal, nonEnumerable } from 'ext:rustyscript/rustyscript.js';
applyToGlobal({
    console: nonEnumerable(
      new _console.Console((msg, level) => globalThis.Deno.core.print(msg, level > 1)),
    ),
});

_console.setNoColorFns(
    () => globalThis.Deno.core.ops.op_bootstrap_no_color() || !globalThis.Deno.core.ops.op_bootstrap_is_stdout_tty(),
    () => globalThis.Deno.core.ops.op_bootstrap_no_color() || !globalThis.Deno.core.ops.op_bootstrap_is_stderr_tty(),
);