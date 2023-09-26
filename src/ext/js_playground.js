export function load() {
    globalThis.js_playground = {
        'register_entrypoint': (f) => Deno.core.ops.op_register_entrypoint(f)
    };
}

load()