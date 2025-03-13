// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use deno_error::JsErrorBox;
use deno_resolver::npm::DenoInNpmPackageChecker;
use deno_runtime::deno_fs;
use node_resolver::analyze::CjsAnalysis as ExtNodeCjsAnalysis;
use node_resolver::analyze::CjsAnalysisExports;
use node_resolver::DenoIsBuiltInNodeModuleChecker;
use serde::Deserialize;
use serde::Serialize;
use sys_traits::impls::RealSys;

use super::resolvers::RustyNpmPackageFolderResolver;
use super::resolvers::RustyResolver;

pub type NodeCodeTranslator = node_resolver::analyze::NodeCodeTranslator<
    RustyCjsCodeAnalyzer,
    DenoInNpmPackageChecker,
    DenoIsBuiltInNodeModuleChecker,
    RustyNpmPackageFolderResolver,
    RealSys,
>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CjsAnalysis {
    /// The module was found to be an ES module.
    Esm,
    /// The module was CJS.
    Cjs {
        exports: Vec<String>,
        reexports: Vec<String>,
    },
}
impl From<ExtNodeCjsAnalysis<'_>> for CjsAnalysis {
    fn from(analysis: ExtNodeCjsAnalysis) -> Self {
        match analysis {
            ExtNodeCjsAnalysis::Esm(_) => CjsAnalysis::Esm,
            ExtNodeCjsAnalysis::Cjs(analysis) => CjsAnalysis::Cjs {
                exports: analysis.exports,
                reexports: analysis.reexports,
            },
        }
    }
}
impl From<deno_ast::CjsAnalysis> for CjsAnalysis {
    fn from(analysis: deno_ast::CjsAnalysis) -> Self {
        Self::Cjs {
            exports: analysis.exports,
            reexports: analysis.reexports,
        }
    }
}

pub struct RustyCjsCodeAnalyzer {
    fs: deno_fs::FileSystemRc,
    cache: RefCell<HashMap<String, CjsAnalysis>>,
    cjs_tracker: Arc<RustyResolver>,
}

impl RustyCjsCodeAnalyzer {
    pub fn new(fs: deno_fs::FileSystemRc, cjs_tracker: Arc<RustyResolver>) -> Self {
        Self {
            fs,
            cache: RefCell::new(HashMap::new()),
            cjs_tracker,
        }
    }

    fn inner_cjs_analysis(
        &self,
        specifier: &ModuleSpecifier,
        source: &str,
    ) -> Result<CjsAnalysis, JsErrorBox> {
        if let Some(analysis) = self.cache.borrow().get(specifier.as_str()) {
            return Ok(analysis.clone());
        }

        let media_type = MediaType::from_specifier(specifier);
        if media_type == MediaType::Json {
            return Ok(CjsAnalysis::Cjs {
                exports: vec![],
                reexports: vec![],
            });
        }

        let parsed_source = deno_ast::parse_program(deno_ast::ParseParams {
            specifier: specifier.clone(),
            text: source.into(),
            media_type,
            capture_tokens: true,
            scope_analysis: false,
            maybe_syntax: None,
        })
        .map_err(JsErrorBox::from_err)?;
        let is_script = parsed_source.compute_is_script();
        let is_cjs = self
            .cjs_tracker
            .is_cjs(parsed_source.specifier(), media_type, is_script);
        let analysis = if is_cjs {
            parsed_source.analyze_cjs().into()
        } else {
            CjsAnalysis::Esm
        };

        self.cache
            .borrow_mut()
            .insert(specifier.as_str().to_string(), analysis.clone());

        Ok(analysis)
    }

    fn analyze_cjs<'a>(
        &self,
        specifier: &ModuleSpecifier,
        source: Cow<'a, str>,
    ) -> Result<ExtNodeCjsAnalysis<'a>, JsErrorBox> {
        let analysis = self.inner_cjs_analysis(specifier, &source)?;
        match analysis {
            CjsAnalysis::Esm => Ok(ExtNodeCjsAnalysis::Esm(source)),
            CjsAnalysis::Cjs { exports, reexports } => {
                Ok(ExtNodeCjsAnalysis::Cjs(CjsAnalysisExports {
                    exports,
                    reexports,
                }))
            }
        }
    }
}

#[async_trait::async_trait(?Send)]
impl node_resolver::analyze::CjsCodeAnalyzer for RustyCjsCodeAnalyzer {
    async fn analyze_cjs<'a>(
        &self,
        specifier: &ModuleSpecifier,
        source: Option<Cow<'a, str>>,
    ) -> Result<ExtNodeCjsAnalysis<'a>, JsErrorBox> {
        let source = match source {
            Some(source) => source,
            None => {
                if let Ok(path) = specifier.to_file_path() {
                    if let Ok(source_from_file) =
                        self.fs.read_text_file_lossy_async(path, None).await
                    {
                        source_from_file
                    } else {
                        return Ok(ExtNodeCjsAnalysis::Cjs(CjsAnalysisExports {
                            exports: vec![],
                            reexports: vec![],
                        }));
                    }
                } else {
                    return Ok(ExtNodeCjsAnalysis::Cjs(CjsAnalysisExports {
                        exports: vec![],
                        reexports: vec![],
                    }));
                }
            }
        };

        self.analyze_cjs(specifier, source)
    }
}
