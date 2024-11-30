//
// This file is a whitelist of ops provided by the deno runtime
// This whitelist is used in tests to confirm that extensions
// marked as sandboxed are in-fact safe.
// 
// It is not auto-generated, and is maintained manually.
//

export const whitelist = {
    //
    // Known dangerous ops
    // Default ops that break sandbox - all of these have been disabled by default
    //
    "op_panic": "Core - BREAKING - Stubbed out in rustyscript",

    //
    // Core ops
    // All core ops MUST preserve the sandbox
    //

    "op_import_sync": "Harmless",
    "op_get_extras_binding_object": "Harmless",
    
    "op_leak_tracing_get": "Harmless",
    "op_leak_tracing_get_all": "Harmless",
    "op_leak_tracing_submit": "Harmless",
    "op_leak_tracing_enable": "Harmless",
    "op_add": "Harmless",
    "op_add_async": "Harmless",

    "op_close": "ResourceTable; Requires rust-side additions to be breaking",
    "op_try_close": "ResourceTable; Requires rust-side additions to be breaking",
    "op_print": "ResourceTable; Requires rust-side additions to be breaking",
    "op_resources": "ResourceTable; Requires rust-side additions to be breaking",
    "op_wasm_streaming_feed": "ResourceTable; Requires rust-side additions to be breaking",
    "op_wasm_streaming_set_url": "ResourceTable; Requires rust-side additions to be breaking",
    "op_void_sync": "ResourceTable; Requires rust-side additions to be breaking",
    "op_error_async": "ResourceTable; Requires rust-side additions to be breaking",
    "op_error_async_deferred": "ResourceTable; Requires rust-side additions to be breaking",
    "op_void_async": "ResourceTable; Requires rust-side additions to be breaking",
    "op_void_async_deferred": "ResourceTable; Requires rust-side additions to be breaking",
    "op_read": "ResourceTable; Requires rust-side additions to be breaking",
    "op_read_all": "ResourceTable; Requires rust-side additions to be breaking",
    "op_write": "ResourceTable; Requires rust-side additions to be breaking",
    "op_read_sync": "ResourceTable; Requires rust-side additions to be breaking",
    "op_write_sync": "ResourceTable; Requires rust-side additions to be breaking",
    "op_write_all": "ResourceTable; Requires rust-side additions to be breaking",
    "op_write_type_error": "ResourceTable; Requires rust-side additions to be breaking",
    "op_shutdown": "ResourceTable; Requires rust-side additions to be breaking",
    "op_cancel_handle": "ResourceTable; Requires rust-side additions to be breaking",

    "op_encode_binary_string": "Encoding",
    "op_format_file_name": "Encoding",
    "op_str_byte_length": "Encoding",

    "op_is_terminal": "Type checking",
    "op_is_any_array_buffer": "Type checking",
    "op_is_arguments_object": "Type checking",
    "op_is_array_buffer": "Type checking",
    "op_is_array_buffer_view": "Type checking",
    "op_is_async_function": "Type checking",
    "op_is_big_int_object": "Type checking",
    "op_is_boolean_object": "Type checking",
    "op_is_boxed_primitive": "Type checking",
    "op_is_data_view": "Type checking",
    "op_is_date": "Type checking",
    "op_is_generator_function": "Type checking",
    "op_is_generator_object": "Type checking",
    "op_is_map": "Type checking",
    "op_is_map_iterator": "Type checking",
    "op_is_module_namespace_object": "Type checking",
    "op_is_native_error": "Type checking",
    "op_is_number_object": "Type checking",
    "op_is_promise": "Type checking",
    "op_is_proxy": "Type checking",
    "op_is_reg_exp": "Type checking",
    "op_is_set": "Type checking",
    "op_is_set_iterator": "Type checking",
    "op_is_shared_array_buffer": "Type checking",
    "op_is_string_object": "Type checking",
    "op_is_symbol_object": "Type checking",
    "op_is_typed_array": "Type checking",
    "op_is_weak_map": "Type checking",
    "op_is_weak_set": "Type checking",

    //
    // Rustyscript
    // Provided by us, so we can trust them
    "op_register_entrypoint": "Rustyscript builtin",
    "call_registered_function": "Rustyscript builtin",
    "call_registered_function_async": "Rustyscript builtin",
    "op_panic2": "Panic stub to replace op_panic",

    //
    // v8 ops
    // These are provided by v8, and are therefore core
    // All v8 ops MUST preserve the sandbox
    "op_add_main_module_handler": "V8 op - non breaking",
    "op_set_handled_promise_rejection_handler": "V8 op - non breaking",
    "op_timer_queue": "V8 op - non breaking",
    "op_timer_queue_system": "V8 op - non breaking",
    "op_timer_queue_immediate": "V8 op - non breaking",
    "op_timer_cancel": "V8 op - non breaking",
    "op_timer_ref": "V8 op - non breaking",
    "op_timer_unref": "V8 op - non breaking",
    "op_ref_op": "V8 op - non breaking",
    "op_unref_op": "V8 op - non breaking",
    "op_lazy_load_esm": "V8 op - non breaking",
    "op_run_microtasks": "V8 op - non breaking",
    "op_has_tick_scheduled": "V8 op - non breaking",
    "op_set_has_tick_scheduled": "V8 op - non breaking",
    "op_eval_context": "V8 op - non breaking",
    "op_queue_microtask": "V8 op - non breaking",
    "op_encode": "V8 op - non breaking",
    "op_decode": "V8 op - non breaking",
    "op_serialize": "V8 op - non breaking",
    "op_deserialize": "V8 op - non breaking",
    "op_set_promise_hooks": "V8 op - non breaking",
    "op_get_promise_details": "V8 op - non breaking",
    "op_get_proxy_details": "V8 op - non breaking",
    "op_get_non_index_property_names": "V8 op - non breaking",
    "op_get_constructor_name": "V8 op - non breaking",
    "op_memory_usage": "V8 op - non breaking",
    "op_set_wasm_streaming_callback": "V8 op - non breaking",
    "op_abort_wasm_streaming": "V8 op - non breaking",
    "op_destructure_error": "V8 op - non breaking",
    "op_dispatch_exception": "V8 op - non breaking",
    "op_op_names": "V8 op - non breaking",
    "op_apply_source_map": "V8 op - non breaking",
    "op_apply_source_map_filename": "V8 op - non breaking",
    "op_set_call_site_evals": "V8 op - non breaking",
    "op_current_user_call_site": "V8 op - non breaking",
    "op_set_format_exception_callback": "V8 op - non breaking",
    "op_event_loop_has_more_work": "V8 op - non breaking",

    //
    // Cache    
    // Preserves sandbox: NO
    "op_cache_storage_open": "deno_cache: exempt",
    "op_cache_storage_has": "deno_cache: exempt",
    "op_cache_storage_delete": "deno_cache: exempt",
    "op_cache_put": "deno_cache: exempt",
    "op_cache_match": "deno_cache: exempt",
    "op_cache_delete": "deno_cache: exempt",

    //
    // Console
    // Preserves sandbox: YES
    "op_preview_entries": "deno_console",

    //
    // Crypto
    // Preserves sandbox: YES
    "op_crypto_get_random_values": "deno_crypto",
    "op_crypto_generate_key": "deno_crypto",
    "op_crypto_sign_key": "deno_crypto",
    "op_crypto_verify_key": "deno_crypto",
    "op_crypto_derive_bits": "deno_crypto",
    "op_crypto_import_key": "deno_crypto",
    "op_crypto_export_key": "deno_crypto",
    "op_crypto_encrypt": "deno_crypto",
    "op_crypto_decrypt": "deno_crypto",
    "op_crypto_subtle_digest": "deno_crypto",
    "op_crypto_random_uuid": "deno_crypto",
    "op_crypto_wrap_key": "deno_crypto",
    "op_crypto_unwrap_key": "deno_crypto",
    "op_crypto_base64url_decode": "deno_crypto",
    "op_crypto_base64url_encode": "deno_crypto",
    "op_crypto_generate_x25519_keypair": "deno_crypto",
    "op_crypto_derive_bits_x25519": "deno_crypto",
    "op_crypto_import_spki_x25519": "deno_crypto",
    "op_crypto_import_pkcs8_x25519": "deno_crypto",
    "op_crypto_generate_ed25519_keypair": "deno_crypto",
    "op_crypto_import_spki_ed25519": "deno_crypto",
    "op_crypto_import_pkcs8_ed25519": "deno_crypto",
    "op_crypto_sign_ed25519": "deno_crypto",
    "op_crypto_verify_ed25519": "deno_crypto",
    "op_crypto_export_spki_ed25519": "deno_crypto",
    "op_crypto_export_pkcs8_ed25519": "deno_crypto",
    "op_crypto_jwk_x_ed25519": "deno_crypto",
    "op_crypto_export_spki_x25519": "deno_crypto",
    "op_crypto_export_pkcs8_x25519": "deno_crypto",
    "op_crypto_generate_x448_keypair": "deno_crypto",
    "op_crypto_derive_bits_x448": "deno_crypto",
    "op_crypto_import_spki_x448": "deno_crypto",
    "op_crypto_import_pkcs8_x448": "deno_crypto",
    "op_crypto_export_spki_x448": "deno_crypto",
    "op_crypto_export_pkcs8_x448": "deno_crypto",

    //
    // IO + TTY
    // Preserves sandbox: NO
    "op_set_raw": "TTY: exempt",
    "op_console_size": "TTY: exempt",
    "op_read_line_prompt": "TTY: exempt",

    //
    // Url
    // Preserves sandbox: YES
    "op_url_reparse": "deno_url",
    "op_url_parse": "deno_url",
    "op_url_get_serialization": "deno_url",
    "op_url_parse_with_base": "deno_url",
    "op_url_parse_search_params": "deno_url",
    "op_url_stringify_search_params": "deno_url",
    "op_urlpattern_parse": "deno_url",
    "op_urlpattern_process_match_input": "deno_url",

    //
    // Web + Fetch + Net
    // Preserves sandbox: NO
    //

    "op_base64_decode": "deno_web: exempt",
    "op_base64_encode": "deno_web: exempt",
    "op_base64_atob": "deno_web: exempt",
    "op_base64_btoa": "deno_web: exempt",
    "op_base64_write": "deno_web: exempt",
    "op_encoding_normalize_label": "deno_web: exempt",
    "op_encoding_decode_single": "deno_web: exempt",
    "op_encoding_decode_utf8": "deno_web: exempt",
    "op_encoding_new_decoder": "deno_web: exempt",
    "op_encoding_decode": "deno_web: exempt",
    "op_encoding_encode_into": "deno_web: exempt",
    "op_blob_create_part": "deno_web: exempt",
    "op_blob_slice_part": "deno_web: exempt",
    "op_blob_read_part": "deno_web: exempt",
    "op_blob_remove_part": "deno_web: exempt",
    "op_blob_create_object_url": "deno_web: exempt",
    "op_blob_revoke_object_url": "deno_web: exempt",
    "op_blob_from_object_url": "deno_web: exempt",
    "op_message_port_create_entangled": "deno_web: exempt",
    "op_message_port_post_message": "deno_web: exempt",
    "op_message_port_recv_message": "deno_web: exempt",
    "op_message_port_recv_message_sync": "deno_web: exempt",
    "op_compression_new": "deno_web: exempt",
    "op_compression_write": "deno_web: exempt",
    "op_compression_finish": "deno_web: exempt",
    "op_now": "deno_web: exempt",
    "op_defer": "deno_web: exempt",
    "op_readable_stream_resource_allocate": "deno_web: exempt",
    "op_readable_stream_resource_allocate_sized": "deno_web: exempt",
    "op_readable_stream_resource_get_sink": "deno_web: exempt",
    "op_readable_stream_resource_write_error": "deno_web: exempt",
    "op_readable_stream_resource_write_buf": "deno_web: exempt",
    "op_readable_stream_resource_write_sync": "deno_web: exempt",
    "op_readable_stream_resource_close": "deno_web: exempt",
    "op_readable_stream_resource_await_close": "deno_web: exempt",

    "op_fetch": "deno_fetch: exempt",
    "op_fetch_send": "deno_fetch: exempt",
    "op_fetch_response_upgrade": "deno_fetch: exempt",
    "op_utf8_to_byte_string": "deno_fetch: exempt",
    "op_fetch_custom_client": "deno_fetch: exempt",

    "op_net_accept_tcp": "deno_net: exempt",
    "op_net_connect_tcp": "deno_net: exempt",
    "op_net_listen_tcp": "deno_net: exempt",
    "op_net_listen_udp": "deno_net: exempt",
    "op_node_unstable_net_listen_udp": "deno_net: exempt",
    "op_net_recv_udp": "deno_net: exempt",
    "op_net_send_udp": "deno_net: exempt",
    "op_net_join_multi_v4_udp": "deno_net: exempt",
    "op_net_join_multi_v6_udp": "deno_net: exempt",
    "op_net_leave_multi_v4_udp": "deno_net: exempt",
    "op_net_leave_multi_v6_udp": "deno_net: exempt",
    "op_net_set_multi_loopback_udp": "deno_net: exempt",
    "op_net_set_multi_ttl_udp": "deno_net: exempt",
    "op_dns_resolve": "deno_net: exempt",
    "op_set_nodelay": "deno_net: exempt",
    "op_set_keepalive": "deno_net: exempt",
    "op_tls_key_null": "deno_net: exempt",
    "op_tls_key_static": "deno_net: exempt",
    "op_tls_key_static_from_file": "deno_net: exempt",
    "op_tls_cert_resolver_create": "deno_net: exempt",
    "op_tls_cert_resolver_poll": "deno_net: exempt",
    "op_tls_cert_resolver_resolve": "deno_net: exempt",
    "op_tls_cert_resolver_resolve_error": "deno_net: exempt",
    "op_tls_start": "deno_net: exempt",
    "op_net_connect_tls": "deno_net: exempt",
    "op_net_listen_tls": "deno_net: exempt",
    "op_net_accept_tls": "deno_net: exempt",
    "op_tls_handshake": "deno_net: exempt",
    "op_net_accept_unix": "deno_net: exempt",
    "op_net_connect_unix": "deno_net: exempt",
    "op_net_listen_unix": "deno_net: exempt",
    "op_net_listen_unixpacket": "deno_net: exempt",
    "op_node_unstable_net_listen_unixpacket": "deno_net: exempt",
    "op_net_recv_unixpacket": "deno_net: exempt",
    "op_net_send_unixpacket": "deno_net: exempt",

    //
    // Websocket
    // Preserves sandbox: NO
    "op_ws_check_permission_and_cancel_handle": "deno_websocket: exempt",
    "op_ws_create": "deno_websocket: exempt",
    "op_ws_close": "deno_websocket: exempt",
    "op_ws_next_event": "deno_websocket: exempt",
    "op_ws_get_buffer": "deno_websocket: exempt",
    "op_ws_get_buffer_as_string": "deno_websocket: exempt",
    "op_ws_get_error": "deno_websocket: exempt",
    "op_ws_send_binary": "deno_websocket: exempt",
    "op_ws_send_binary_ab": "deno_websocket: exempt",
    "op_ws_send_text": "deno_websocket: exempt",
    "op_ws_send_binary_async": "deno_websocket: exempt",
    "op_ws_send_text_async": "deno_websocket: exempt",
    "op_ws_send_ping": "deno_websocket: exempt",
    "op_ws_get_buffered_amount": "deno_websocket: exempt",

    //
    // Webstorage
    // Preserves sandbox: NO
    "op_webstorage_length": "deno_webstorage: exempt",
    "op_webstorage_key": "deno_webstorage: exempt",
    "op_webstorage_set": "deno_webstorage: exempt",
    "op_webstorage_get": "deno_webstorage: exempt",
    "op_webstorage_remove": "deno_webstorage: exempt",
    "op_webstorage_clear": "deno_webstorage: exempt",
    "op_webstorage_iterate_keys": "deno_webstorage: exempt",
}