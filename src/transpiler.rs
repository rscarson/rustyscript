// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.
//! This file transpiles TypeScript and JSX/TSX
//! modules.
//!
//! It will only transpile, not typecheck (like Deno's `--no-check` flag).
use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceTextInfo;
use deno_core::anyhow::anyhow;
use deno_core::anyhow::bail;
use deno_core::anyhow::Error;
use deno_core::ModuleSpecifier;
use deno_core::ModuleType;

pub fn transpile(module_specifier: &ModuleSpecifier, code: &str) -> Result<String, Error> {
    let path = module_specifier
        .to_file_path()
        .map_err(|_| anyhow!("Only file:// URLs are supported."))?;
    let media_type = MediaType::from_path(&path);
    let (_module_type, should_transpile) = match MediaType::from_path(&path) {
        MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => (ModuleType::JavaScript, false),
        MediaType::Jsx => (ModuleType::JavaScript, true),
        MediaType::TypeScript
        | MediaType::Mts
        | MediaType::Cts
        | MediaType::Dts
        | MediaType::Dmts
        | MediaType::Dcts
        | MediaType::Tsx => (ModuleType::JavaScript, true),
        MediaType::Json => (ModuleType::Json, false),
        _ => bail!("Unknown extension {:?}", path.extension()),
    };

    let code = if should_transpile {
        let parsed = deno_ast::parse_module(ParseParams {
            specifier: module_specifier.to_string(),
            text_info: SourceTextInfo::from_string(code.to_string()),
            media_type,
            capture_tokens: false,
            scope_analysis: false,
            maybe_syntax: None,
        })?;
        let res = parsed.transpile(&deno_ast::EmitOptions {
            inline_source_map: false,
            source_map: true,
            inline_sources: true,
            ..Default::default()
        })?;
        res.text
    } else {
        code.to_string()
    };

    Ok(code)
}
