# Contributing to rustyscript

Thank you for your interest in contributing to rustyscript! I appreciate your time, and your help in keeping rustyscript useful for everyone.

This document will lay out general instructions for several common tasks, as well as the general structure of the project

## Adding Deno extensions
The deno extensions require extensive configuration on both the rust and JS sides to function, and have a complex dependency structure that
can easily cause unpredictable issues

For that reason, rustyscript provides these extensions pre-configured and initialized to the user as crate-level features.

The process of adding an extension involves the following steps:

1- Find the extensions in [deno/ext](https://github.com/denoland/deno/tree/main/ext)
2- Note the dependencies in `lib.rs`, in the `deps` portion of the call to `deno_core::extension`
3- Add a new crate-feature for the extension in `Cargo.toml`
 - It should be preceded with a comment linking the portion of the spec it implements (see the extensions's readme)
 - It should begin with the extension's crate, which must be an optional dependency of rustyscript
 - It should then list the rustyscript crate-features associated with each of the dependencies found in step 2
4- Add a new directory in `src/ext/extension_name`. It will contain `mod.rs` and `init_extension_name.js`
5- Navigate to [deno/runtime/js](https://github.com/denoland/deno/tree/main/runtime/js) and find all the places in global the extension is added
6- In `init_extension_name.js`, include -all- JS sources provided by the new extension, and add the relevant portions to global (see other exts for examples)
7- In `mod.rs`, create your `init_extension_name` extensions and provide the `extensions` and `snapshot_extensions` functions
8- If the extension requires configuration, add a feature-gated section to `ExtensionOptions`
 - If only one field, add it directly to ExtensionOptions
 - Otherwise add an options structure in `mod.rs` and reference that
9- in `src/ext/mod.rs` add feature-gated imports, and add your extension to the extensions functions -BELOW- any of its dependencies
10- Add a section to the table in `lib.rs` for the new extension, and use `cargo rdme` to regenerate the readme

## Project Structure

There are several public-facing portions of the project, most notably `runtime`, `js_value`, and `module`

But there are also portions that should never be public facing:

### `inner_runtime::InnerRuntime`
This is the underlying logic that runtime wraps - it should be async-only, and exposed only through `runtime`'s calls out to it  
This should be kept private to simplify the APi

### `module_loader::RustyLoader`
This is the logic underlying transpilation, fetching module code, generating errors, and security around FS/URL imports  
This should be kept private due to the sheer number of ways the crate depends on the behaviour of the loader.

### `transpiler`
This is simply a set of utilities for TS -> JS transpilation and is only useful in the context of the module-loader  
There is no reason to expose this to the user

### `ext::*`
The deno extensions require extensive configuration on both the rust and JS sides to function, and have a complex dependency structure that
can easily cause unpredictable issues

For that reason, rustyscript provides these extensions pre-configured and initialized to the user as crate-level features. Because we managed load-order
and dependencies between extensions, exposing these could be dangerous.