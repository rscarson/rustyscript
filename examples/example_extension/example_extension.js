/**
 * This file is the JS component used in the runtime_extensions example
 * It exports a function that calls the op_add_example Rust function
 */

export const add = (a, b) => Deno.core.ops.op_add_example(a, b);
