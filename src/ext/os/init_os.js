const core = globalThis.Deno.core;

function exit(code = 0) {
    if (typeof code !== "number" || !Number.isInteger(code)) {
        throw new TypeError("Exit code must be an integer");
    }
    
    core.ops.op_rustyscript_exit(code);
}

// Make exit available on the global Deno object
if (typeof globalThis.Deno === "undefined") {
    globalThis.Deno = {};
}

globalThis.Deno.exit = exit;

export { exit };