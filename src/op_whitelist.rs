//! A module for managing the whitelist of ops.
//!
//! This is a security feature designed to catch potentially unsafe operations and ensure they are reviewed before use.

macro_rules! whitelist {
    ($( $src:literal => [ stubs = [$($stubname:ident $(,)?),*], $($safename:ident),* $(,)?]),+ $(,)?) => {
        /// A manually curated whitelist of deno OP2s
        ///
        /// These have all been reviewed and approved by the author as being sandbox-preserving.
        ///
        /// Any Ops marked stubbed were unsafe, but have been made safe for use.
        const WHITELIST: OpWhitelist = OpWhitelist::new(&[ $( OpSrc::new($src, &[ $(stringify!($safename)),* ], &[$(stringify!($stubname)),* ]) ),+ ]);
    };
}

/// Get the global whitelist of ops.
pub fn get_whitelist() -> &'static OpWhitelist {
    &WHITELIST
}

whitelist!(
    "rustyscript" => [
        stubs = [],

        op_register_entrypoint,
        call_registered_function,
        call_registered_function_async,
        op_panic2,
    ],
    "deno_core" => [
        stubs = [ op_panic ],

        op_structured_clone,
        op_import_sync,
        op_get_extras_binding_object,
        op_leak_tracing_get,
        op_leak_tracing_get_all,
        op_leak_tracing_submit,
        op_leak_tracing_enable,
        op_add,
        op_add_async,
        op_close,
        op_try_close,
        op_print,
        op_resources,
        op_wasm_streaming_feed,
        op_wasm_streaming_set_url,
        op_void_sync,
        op_error_async,
        op_error_async_deferred,
        op_void_async,
        op_void_async_deferred,
        op_read,
        op_read_all,
        op_write,
        op_read_sync,
        op_write_sync,
        op_write_all,
        op_write_type_error,
        op_shutdown,
        op_cancel_handle,
        op_encode_binary_string,
        op_format_file_name,
        op_str_byte_length,
        op_is_terminal,
        op_is_any_array_buffer,
        op_is_arguments_object,
        op_is_array_buffer,
        op_is_array_buffer_view,
        op_is_async_function,
        op_is_big_int_object,
        op_is_boolean_object,
        op_is_boxed_primitive,
        op_is_data_view,
        op_is_date,
        op_is_generator_function,
        op_is_generator_object,
        op_is_map,
        op_is_map_iterator,
        op_is_module_namespace_object,
        op_is_native_error,
        op_is_number_object,
        op_is_promise,
        op_is_proxy,
        op_is_reg_exp,
        op_is_set,
        op_is_set_iterator,
        op_is_shared_array_buffer,
        op_is_string_object,
        op_is_symbol_object,
        op_is_typed_array,
        op_is_weak_map,
        op_is_weak_set,
        op_encode_binary_string,
        op_format_file_name,
        op_str_byte_length,
        op_is_terminal,
        op_is_any_array_buffer,
        op_is_arguments_object,
        op_is_array_buffer,
        op_is_array_buffer_view,
        op_is_async_function,
        op_is_big_int_object,
        op_is_boolean_object,
        op_is_boxed_primitive,
        op_is_data_view,
        op_is_date,
        op_is_generator_function,
        op_is_generator_object,
        op_is_map,
        op_is_map_iterator,
        op_is_module_namespace_object,
        op_is_native_error,
        op_is_number_object,
        op_is_promise,
        op_is_proxy,
        op_is_reg_exp,
        op_is_set,
        op_is_set_iterator,
        op_is_shared_array_buffer,
        op_is_string_object,
        op_is_symbol_object,
        op_is_typed_array,
        op_is_weak_map,
        op_is_weak_set,
    ],
    "v8" => [
        stubs = [],

        op_add_main_module_handler,
        op_set_handled_promise_rejection_handler,
        op_timer_queue,
        op_timer_queue_system,
        op_timer_queue_immediate,
        op_timer_cancel,
        op_timer_ref,
        op_timer_unref,
        op_ref_op,
        op_unref_op,
        op_lazy_load_esm,
        op_run_microtasks,
        op_has_tick_scheduled,
        op_set_has_tick_scheduled,
        op_eval_context,
        op_queue_microtask,
        op_encode,
        op_decode,
        op_serialize,
        op_deserialize,
        op_set_promise_hooks,
        op_get_promise_details,
        op_get_proxy_details,
        op_get_non_index_property_names,
        op_get_constructor_name,
        op_memory_usage,
        op_set_wasm_streaming_callback,
        op_abort_wasm_streaming,
        op_destructure_error,
        op_dispatch_exception,
        op_op_names,
        op_apply_source_map,
        op_apply_source_map_filename,
        op_set_call_site_evals,
        op_current_user_call_site,
        op_set_format_exception_callback,
        op_event_loop_has_more_work,
        op_get_ext_import_meta_proto,
    ],
    "deno_console" => [
        stubs = [],

        op_preview_entries,
    ],
    "deno_crypto" => [
        stubs = [],

        op_crypto_get_random_values,
        op_crypto_generate_key,
        op_crypto_sign_key,
        op_crypto_verify_key,
        op_crypto_derive_bits,
        op_crypto_import_key,
        op_crypto_export_key,
        op_crypto_encrypt,
        op_crypto_decrypt,
        op_crypto_subtle_digest,
        op_crypto_random_uuid,
        op_crypto_wrap_key,
        op_crypto_unwrap_key,
        op_crypto_base64url_decode,
        op_crypto_base64url_encode,
        op_crypto_generate_x25519_keypair,
        op_crypto_derive_bits_x25519,
        op_crypto_import_spki_x25519,
        op_crypto_import_pkcs8_x25519,
        op_crypto_generate_ed25519_keypair,
        op_crypto_import_spki_ed25519,
        op_crypto_import_pkcs8_ed25519,
        op_crypto_sign_ed25519,
        op_crypto_verify_ed25519,
        op_crypto_export_spki_ed25519,
        op_crypto_export_pkcs8_ed25519,
        op_crypto_jwk_x_ed25519,
        op_crypto_export_spki_x25519,
        op_crypto_export_pkcs8_x25519,
        op_crypto_generate_x448_keypair,
        op_crypto_derive_bits_x448,
        op_crypto_import_spki_x448,
        op_crypto_import_pkcs8_x448,
        op_crypto_export_spki_x448,
        op_crypto_export_pkcs8_x448,
        op_crypto_x25519_public_key,
    ],
    "deno_url" => [
        stubs = [],

        op_url_reparse,
        op_url_parse,
        op_url_get_serialization,
        op_url_parse_with_base,
        op_url_parse_search_params,
        op_url_stringify_search_params,
        op_urlpattern_parse,
        op_urlpattern_process_match_input,
    ],
    "web_stub" => [
        stubs = [],
        op_now,
        op_defer,
        op_base64_decode,
        op_base64_atob,
        op_base64_encode,
        op_base64_btoa,
    ],
);

/// A manually curated whitelist of ops.
pub struct OpWhitelist(&'static [OpSrc]);
impl OpWhitelist {
    /// Create a new list of safe ops.
    pub const fn new(ops: &'static [OpSrc]) -> Self {
        OpWhitelist(ops)
    }

    /// Check if the whitelist contains a specific op.
    pub fn contains_op(&self, op: &str) -> bool {
        self.0.iter().any(|src| src.contains_op(op))
    }

    /// Get a list of all unsafe ops in a runtime
    pub fn unsafe_ops(&self, rt: &mut crate::Runtime) -> Vec<&str> {
        rt.deno_runtime()
            .op_names()
            .into_iter()
            .filter(|op| !self.contains_op(op))
            .collect()
    }
}

/// A known source of OPs - usually a deno extension
pub struct OpSrc {
    /// Crate or extension name
    pub src: &'static str,

    /// Known safe ops
    pub ops: &'static [&'static str],

    /// Known unsafe ops that have been stubbed out and replaced
    pub stubs: &'static [&'static str],
}
impl OpSrc {
    /// Create a new OpSrc.
    pub const fn new(
        src: &'static str,
        ops: &'static [&'static str],
        stubs: &'static [&'static str],
    ) -> Self {
        OpSrc { src, ops, stubs }
    }

    /// Check if the OpSrc contains a specific op.
    pub fn contains_op(&self, op: &str) -> bool {
        self.ops.contains(&op) || self.stubs.contains(&op)
    }
}
