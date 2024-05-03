// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
//! This file transpiles TypeScript and JSX/TSX
//! modules.
//!
//! It will only transpile, not typecheck (like Deno's `--no-check` flag).

use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceTextInfo;
use deno_core::anyhow::Error;
use deno_core::ExtensionFileSource;
use deno_core::ExtensionFileSourceCode;
use deno_core::ModuleSpecifier;

use crate::traits::ToModuleSpecifier;

fn should_transpile(media_type: &MediaType) -> bool {
    match media_type {
        MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs | MediaType::Json => false,

        MediaType::Jsx => true,
        MediaType::TypeScript
        | MediaType::Mts
        | MediaType::Cts
        | MediaType::Dts
        | MediaType::Dmts
        | MediaType::Dcts
        | MediaType::Tsx => true,

        _ => false,
    }
}

///
/// Transpiles source code from TS to JS without typechecking
pub fn transpile(module_specifier: &ModuleSpecifier, code: &str) -> Result<String, Error> {
    let media_type = MediaType::from_specifier(module_specifier);
    let should_transpile = should_transpile(&media_type);

    let code = if should_transpile {
        let parsed = deno_ast::parse_module(ParseParams {
            specifier: module_specifier.clone(),
            text_info: SourceTextInfo::from_string(code.to_string()),
            media_type,
            capture_tokens: false,
            scope_analysis: false,
            maybe_syntax: None,
        })?;

        let transpile_options = deno_ast::TranspileOptions {
            ..Default::default()
        };

        let emit_options = deno_ast::EmitOptions {
            keep_comments: true,
            source_map: deno_ast::SourceMapOption::Inline,
            inline_sources: true,
            ..Default::default()
        };

        let res = parsed.transpile(&transpile_options, &emit_options)?;
        res.into_source().text
    } else {
        code.to_string()
    };

    Ok(code)
}

///
/// Transpile an extension
pub fn transpile_extension(source: &mut ExtensionFileSource) -> Result<(), Error> {
    let specifier = source.specifier.to_module_specifier()?;
    let media_type = MediaType::from_specifier(&specifier);
    if should_transpile(&media_type) {
        let source_code = match &source.code {
            #[allow(deprecated)]
            ExtensionFileSourceCode::IncludedInBinary(s) => s.to_string(),

            ExtensionFileSourceCode::LoadedFromFsDuringSnapshot(s) => s.to_string(),
            ExtensionFileSourceCode::Computed(s) => s.to_string(),
            ExtensionFileSourceCode::LoadedFromMemoryDuringSnapshot(s) => s.to_string(),
        };
        let source_code = transpile(&specifier, &source_code)?;
        source.code = ExtensionFileSourceCode::Computed(source_code.into());
    }

    Ok(())
}
