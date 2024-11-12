import {denoNs} from 'ext:runtime/90_deno_ns.js';

import * as util from 'ext:runtime/06_util.js';
import * as permissions from 'ext:runtime/10_permissions.js';
import * as workers from 'ext:runtime/11_workers.js';
import * as os from 'ext:runtime/30_os.js';
import * as process from 'ext:runtime/40_process.js';
import * as prompt from 'ext:runtime/41_prompt.js';
import * as scope from 'ext:runtime/98_global_scope_shared.js';
import * as scopeWorker from 'ext:runtime/98_global_scope_worker.js';
import * as scopeWindow from 'ext:runtime/98_global_scope_window.js';

const opArgs = scopeWindow.memoizeLazy(() => core.ops.op_bootstrap_args());
const opPid = scopeWindow.memoizeLazy(() => core.ops.op_bootstrap_pid());

import { core, primordials } from "ext:core/mod.js";
primordials.ObjectDefineProperties(denoNs, {
    pid: core.propGetterOnly(opPid),

    // `ppid` should not be memoized.
    // https://github.com/denoland/deno/issues/23004
    ppid: core.propGetterOnly(() => core.ops.op_ppid()),
    noColor: core.propGetterOnly(() => core.ops.op_bootstrap_no_color()),
    args: core.propGetterOnly(opArgs),
    mainModule: core.propGetterOnly(() => core.ops.op_main_module()),
    exitCode: {
        __proto__: null,
        get() {
            return os.getExitCode();
        },
        set(value) {
            os.setExitCode(value);
        },
    },

    core: core.propReadOnly(core),
});

primordials.ObjectDefineProperty(globalThis, "Deno", core.propReadOnly(denoNs));

import * as _console from 'ext:deno_console/01_console.js';
_console.setNoColorFns(
    () => globalThis.Deno.core.ops.op_bootstrap_no_color() || !globalThis.Deno.core.ops.op_bootstrap_is_stdout_tty(),
    () => globalThis.Deno.core.ops.op_bootstrap_no_color() || !globalThis.Deno.core.ops.op_bootstrap_is_stderr_tty(),
);
