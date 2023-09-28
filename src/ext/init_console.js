import { Console } from 'ext:deno_console/01_console.js';
const core = globalThis.Deno.core;
globalThis.console = new Console((msg, level) => core.print(msg, level > 1));