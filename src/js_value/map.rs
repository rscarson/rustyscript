use super::V8Value;
use deno_core::v8::{self, GetPropertyNamesArgs, HandleScope};
use serde::Deserialize;

/// A Deserializable javascript object, that can be stored and used later
/// Must live as long as the runtime it was birthed from
///
/// Allows read-only access properties of the object, and convert it to a hashmap
/// (skipping any keys that are not valid UTF-8)
///
/// [Map::get] returns a [crate::js_value::Value] which can be converted to any rust type, including promises or functions
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct Map(V8Value<ObjectTypeChecker>);
impl_v8!(Map, ObjectTypeChecker);
impl_checker!(ObjectTypeChecker, Object, is_object, |e| {
    crate::Error::JsonDecode(format!("Expected an object, found `{}`", e))
});

impl Map {
    /// Gets a value from the map
    /// Warning: If a key is not valid UTF-8, the value may be inaccessible
    pub fn get(&self, key: &str, runtime: &mut crate::Runtime) -> Option<crate::js_value::Value> {
        let mut scope = runtime.deno_runtime().handle_scope();
        self.get_property_by_name(&mut scope, key)
    }

    /// Converts the map to a hashmap
    /// Skips any keys that are not valid UTF-8
    pub fn to_hashmap(
        &self,
        runtime: &mut crate::Runtime,
    ) -> std::collections::HashMap<String, crate::js_value::Value> {
        let mut scope = runtime.deno_runtime().handle_scope();
        self.to_rust_hashmap(&mut scope)
    }

    /// Returns the keys of the map
    /// Warning: If a key is not valid UTF-8, the value may be inaccessible
    pub fn keys(&self, runtime: &mut crate::Runtime) -> Vec<String> {
        let mut scope = runtime.deno_runtime().handle_scope();
        self.get_string_keys(&mut scope)
    }

    /// Returns the number of keys in the map
    /// Skips any keys that are not valid UTF-8
    pub fn len(&self, runtime: &mut crate::Runtime) -> usize {
        let mut scope = runtime.deno_runtime().handle_scope();
        self.get_string_keys(&mut scope).len()
    }

    pub(crate) fn to_rust_hashmap(
        &self,
        scope: &mut HandleScope,
    ) -> std::collections::HashMap<String, crate::js_value::Value> {
        let keys = self.get_string_keys(scope);
        let mut map = std::collections::HashMap::new();

        for name in keys {
            match self.get_property_by_name(scope, &name) {
                Some(value) => map.insert(name, value),
                None => None,
            };
        }

        map
    }

    pub(crate) fn get_property_by_name(
        &self,
        scope: &mut HandleScope,
        name: &str,
    ) -> Option<crate::js_value::Value> {
        let local = self.0.as_local(scope);
        let key = v8::String::new(scope, name).unwrap();
        let value = local.get(scope, key.into())?;

        let value = v8::Global::new(scope, value);
        Some(crate::js_value::Value::from_v8(value).ok()?)
    }

    pub(crate) fn get_string_keys(&self, scope: &mut HandleScope) -> Vec<String> {
        let local = self.0.as_local(scope);
        let mut keys = vec![];

        let v8_keys = local.get_own_property_names(
            scope,
            GetPropertyNamesArgs {
                mode: v8::KeyCollectionMode::OwnOnly,
                property_filter: v8::PropertyFilter::ALL_PROPERTIES,
                index_filter: v8::IndexFilter::IncludeIndices,
                key_conversion: v8::KeyConversionMode::ConvertToString,
            },
        );
        let v8_keys = match v8_keys {
            Some(keys) => keys,
            None => return keys,
        };

        for i in 0..v8_keys.length() {
            let key = v8_keys.get_index(scope, i).unwrap();
            let key = key.to_rust_string_lossy(scope);
            keys.push(key);
        }

        keys
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Module, Runtime};

    #[test]
    fn test_map() {
        let module = Module::new(
            "test.js",
            "
            export const m = { a: 1, b: 2, c: 3, 0: 4 };
        ",
        );

        let mut runtime = Runtime::new(Default::default()).unwrap();
        let handle = runtime.load_module(&module).unwrap();

        let m: Map = runtime.get_value(Some(&handle), "m").expect("oops");
        assert_eq!(m.len(&mut runtime), 4);

        let a = m.get("a", &mut runtime).unwrap();
        let a: usize = a.try_into(&mut runtime).unwrap();
        assert_eq!(a, 1);

        let zero = m.get("0", &mut runtime).unwrap();
        let zero: usize = zero.try_into(&mut runtime).unwrap();
        assert_eq!(zero, 4);
    }
}
