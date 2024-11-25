import * as util from 'ext:runtime/06_util.js';
import * as permissions from 'ext:runtime/10_permissions.js';
import * as workers from 'ext:runtime/11_workers.js';
import * as os from 'ext:runtime/30_os.js';
import * as process from 'ext:runtime/40_process.js';
import * as prompt from 'ext:runtime/41_prompt.js';
import * as scope from 'ext:runtime/98_global_scope_shared.js';
import * as scopeWorker from 'ext:runtime/98_global_scope_worker.js';
import * as scopeWindow from 'ext:runtime/98_global_scope_window.js';
import * as telemetry from "ext:runtime/telemetry.ts";

import * as errors from "ext:runtime/01_errors.js";
import * as version from "ext:runtime/01_version.ts";
import * as signals from "ext:runtime/40_signals.js";
import * as tty from "ext:runtime/40_tty.js";

const opArgs = scopeWindow.memoizeLazy(() => core.ops.op_bootstrap_args());
const opPid = scopeWindow.memoizeLazy(() => core.ops.op_bootstrap_pid());

import { core } from "ext:core/mod.js";
import { applyToDeno, getterOnly, readOnly, nonEnumerable } from "ext:rustyscript/rustyscript.js";

//applyToDeno(denoNs);
applyToDeno({    
    pid: getterOnly(opPid),

    // `ppid` should not be memoized.
    // https://github.com/denoland/deno/issues/23004
    ppid: getterOnly(() => core.ops.op_ppid()),
    noColor: getterOnly(() => core.ops.op_bootstrap_no_color()),
    args: getterOnly(opArgs),
    mainModule: getterOnly(() => core.ops.op_main_module()),
    exitCode: {
        __proto__: null,
        get() {
            return os.getExitCode();
        },
        set(value) {
            os.setExitCode(value);
        },
    },

    telemetry: nonEnumerable(telemetry.telemetry),
    
    Process: nonEnumerable(process.Process),
    run: nonEnumerable(process.run),
    kill: nonEnumerable(process.kill),
    Command: nonEnumerable(process.Command),
    ChildProcess: nonEnumerable(process.ChildProcess),

    isatty: nonEnumerable(tty.isatty),
    consoleSize: nonEnumerable(tty.consoleSize),

    memoryUsage: nonEnumerable(() => op_runtime_memory_usage()),
    version: nonEnumerable(version.version),
    build: nonEnumerable(core.build),
    errors: nonEnumerable(errors.errors),
    
    permissions: nonEnumerable(permissions.permissions),
    Permissions: nonEnumerable(permissions.Permissions),
    PermissionStatus: nonEnumerable(permissions.PermissionStatus),

    addSignalListener: nonEnumerable(signals.addSignalListener),
    removeSignalListener: nonEnumerable(signals.removeSignalListener),
    
    env: nonEnumerable(os.env),
    exit: nonEnumerable(os.exit),
    execPath: nonEnumerable(os.execPath),
    loadavg: nonEnumerable(os.loadavg),
    osRelease: nonEnumerable(os.osRelease),
    osUptime: nonEnumerable(os.osUptime),
    hostname: nonEnumerable(os.hostname),
    systemMemoryInfo: nonEnumerable(os.systemMemoryInfo),
    networkInterfaces: nonEnumerable(os.networkInterfaces),

    gid: nonEnumerable(os.gid),
    uid: nonEnumerable(os.uid),


    core: readOnly(core),
});

import * as _console from 'ext:deno_console/01_console.js';
_console.setNoColorFns(
    () => globalThis.Deno.core.ops.op_bootstrap_no_color() || !globalThis.Deno.core.ops.op_bootstrap_is_stdout_tty(),
    () => globalThis.Deno.core.ops.op_bootstrap_no_color() || !globalThis.Deno.core.ops.op_bootstrap_is_stderr_tty(),
);
