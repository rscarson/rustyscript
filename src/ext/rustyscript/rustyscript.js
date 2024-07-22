// Theses stubs will generate more useful error messages when timers are disabled
globalThis.setTimeout = () => { throw new Error('setTimeout is not enabled in this environment') };
globalThis.setInterval = () => { throw new Error('setInterval is not enabled in this environment') };
globalThis.setImmediate = () => { throw new Error('setImmediate is not enabled in this environment') };
globalThis.clearTimeout = () => { throw new Error('clearTimeout is not enabled in this environment') };
globalThis.clearInterval = () => { throw new Error('clearInterval is not enabled in this environment') };
globalThis.clearImmediate = () => { throw new Error('clearImmediate is not enabled in this environment') };

// Loaders used by other extensions
const ObjectProperties = {
    'nonEnumerable': {writable: true, enumerable: false, configurable: true},
    'readOnly': {writable: false, enumerable: false, configurable: true},
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

// Populate the global object
globalThis.rustyscript = {
    'register_entrypoint': (f) => Deno.core.ops.op_register_entrypoint(f),
    'bail': (msg) => { throw new Error(msg) },
    
    'functions': new Proxy({}, {
        get: function(_target, name) {
            return (...args) => Deno.core.ops.call_registered_function(name, args);
        }
    }),

    'async_functions': new Proxy({}, {
        get: function(_target, name) {
            return (...args) => Deno.core.ops.call_registered_function_async(name, args);
        }
    })
};
Object.freeze(globalThis.rustyscript);

export {
    nonEnumerable, readOnly, writeable, getterOnly, applyToGlobal
};