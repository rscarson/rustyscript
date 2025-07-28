// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
//! This file transpiles TypeScript and JSX/TSX
//! modules.
//!
//! It will only transpile, not typecheck (like Deno's `--no-check` flag).

use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceTextInfo;
use deno_core::anyhow::Error;
use deno_error::JsErrorBox;
use deno_core::FastString;
use deno_core::ModuleSpecifier;
use deno_core::SourceMapData;
use std::borrow::Cow;
use std::rc::Rc;

pub type ModuleContents = (String, Option<SourceMapData>);

fn should_transpile(media_type: MediaType) -> bool {
    matches!(
        media_type,
        MediaType::Jsx
            | MediaType::TypeScript
            | MediaType::Mts
            | MediaType::Cts
            | MediaType::Dts
            | MediaType::Dmts
            | MediaType::Dcts
            | MediaType::Tsx
    )
}

///
/// Transpiles source code from TS to JS without typechecking
pub fn transpile(module_specifier: &ModuleSpecifier, code: &str) -> Result<ModuleContents, Error> {
    let mut media_type = MediaType::from_specifier(module_specifier);

    if media_type == MediaType::Unknown && module_specifier.as_str().contains("/node:") {
        media_type = MediaType::TypeScript;
    }

    let should_transpile = should_transpile(media_type);

    let code = if should_transpile {
        let sti = SourceTextInfo::from_string(code.to_string());
        let text = sti.text();
        let parsed = deno_ast::parse_module(ParseParams {
            specifier: module_specifier.clone(),
            text,
            media_type,
            capture_tokens: false,
            scope_analysis: false,
            maybe_syntax: None,
        })?;

        let transpile_options = deno_ast::TranspileOptions {
            ..Default::default()
        };

        let transpile_mod_options = deno_ast::TranspileModuleOptions {
            ..Default::default()
        };

        let emit_options = deno_ast::EmitOptions {
            remove_comments: false,
            source_map: deno_ast::SourceMapOption::Separate,
            inline_sources: false,
            ..Default::default()
        };
        let res = parsed
            .transpile(&transpile_options, &transpile_mod_options, &emit_options)?
            .into_source();

        let text = res.text;

        let source_map: Option<SourceMapData> = res.source_map.map(|sm| sm.into_bytes().into());

        (text, source_map)
    } else {
        (code.to_string(), None)
    };

    Ok(code)
}

///
/// Transpile an extension
#[allow(clippy::type_complexity)]
pub fn transpile_extension(
    specifier: &ModuleSpecifier,
    code: &str,
) -> Result<(FastString, Option<Cow<'static, [u8]>>), JsErrorBox> {
    let (code, source_map) = transpile(specifier, code).map_err(|e| JsErrorBox::new("Error", e.to_string()))?;
    let code = FastString::from(code);
    Ok((code, source_map))
}

pub type ExtensionTranspiler =
    Rc<dyn Fn(FastString, FastString) -> Result<(FastString, Option<Cow<'static, [u8]>>), JsErrorBox>>;
pub type ExtensionTranspilation = (FastString, Option<Cow<'static, [u8]>>);
