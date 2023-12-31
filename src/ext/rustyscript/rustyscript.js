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

// Loaders used by other extensions
const ObjectProperties = {
    'nonEnumerable': {writable: true, enumerable: false, configurable: true},
    'readOnly': {writable: true, enumerable: false, configurable: true},
    'writeable': {writable: true, enumerable: true, configurable: true},
    'getterOnly': {enumerable: true, configurable: true},

    'apply': (value, type) => {
        return {
            'value': value,
            ...ObjectProperties[type]
        };
    }
}
const nonEnumerable = (value) => ObjectProperties.apply(value, nonEnumerable);
const readOnly = (value) => ObjectProperties.apply(value, readOnly);
const writeable = (value) => ObjectProperties.apply(value, writeable);
const getterOnly = (getter) => {
    return {
        get: getter,
        set() {},
        ...ObjectProperties.getterOnly
    };
}
const applyToGlobal = (properties) => Object.defineProperties(globalThis, properties);

// Set us up for platform resets
globalThis.rustyscript_reset = () => {
    let backup = deepClone(globalThis.rustyscript.global_backup);
    for (const key of Object.keys(globalThis)) {
        if (backup[key]) continue;
        globalThis[key] = undefined;
    }

    for (const key of Object.keys(backup)) {
        globalThis[key] = backup[key];
    }
}

// Populate the global object
globalThis.rustyscript = {
    'register_entrypoint': (f) => Deno.core.ops.op_register_entrypoint(f),
    'bail': (msg) => { throw new Error(msg) },
    
    'global_backup': deepClone(globalThis),
    'functions': new Proxy({}, {
        get: function(_target, name) {
            return (...args) => Deno.core.ops.call_registered_function(name, args);
        }
    })
};
Object.freeze(globalThis.rustyscript);

export {
    nonEnumerable, readOnly, writeable, getterOnly, applyToGlobal
};