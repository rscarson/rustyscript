// https://stackoverflow.com/a/40293777
// structuredClone not available in this context
function deepClone(obj, hash = new WeakMap()) {
    // Do not try to clone primitives or functions
    if (Object(obj) !== obj || obj instanceof Function) return obj;
    if (hash.has(obj)) return hash.get(obj); // Cyclic reference
    try { // Try to run constructor (without arguments, as we don't know them)
        var result = new obj.constructor();
    } catch(e) { // Constructor failed, create object without running the constructor
        result = Object.create(Object.getPrototypeOf(obj));
    }
    // Optional: support for some standard constructors (extend as desired)
    if (obj instanceof Map)
        Array.from(obj, ([key, val]) => result.set(deepClone(key, hash), 
                                                   deepClone(val, hash)) );
    else if (obj instanceof Set)
        Array.from(obj, (key) => result.add(deepClone(key, hash)) );
    // Register in hash    
    hash.set(obj, result);
    // Clone and assign enumerable own properties recursively
    return Object.assign(result, ...Object.keys(obj).map (
        key => ({ [key]: deepClone(obj[key], hash) }) ));
}

globalThis.js_playground_reset = () => {
    let backup = deepClone(globalThis.js_playground.global_backup);
    for (const key of Object.keys(globalThis)) {
        if (backup[key]) continue;
        globalThis[key] = undefined;
    }

    for (const key of Object.keys(backup)) {
        globalThis[key] = backup[key];
    }
}
globalThis.js_playground = {
    'register_entrypoint': (f) => Deno.core.ops.op_register_entrypoint(f),
    'bail': (msg) => { throw new Error(msg) },
    
    'global_backup': deepClone(globalThis),
};
Object.freeze(globalThis.js_playground);