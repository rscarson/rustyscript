/**
 * This file is the JS component used in the runtime_extensions example
 * It adds an object to the global scope which can call the OPs configured
 * for the extension
 */
globalThis.example_ext = {
    'add': (a, b) => Deno.core.ops.op_add_example(a, b)
};