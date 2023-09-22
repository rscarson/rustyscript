//! # Simple deno wrapper for module execution
//! [![Crates.io](https://img.shields.io/crates/v/js-playground.svg)](https://crates.io/crates/js-playground)
//! [![Build Status](https://github.com/rscarson/js-playground/workflows/Rust/badge.svg)](https://github.com/rscarson/js-playground/actions?workflow=Rust)
//! [![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://raw.githubusercontent.com/rscarson/js-playground/master/LICENSE)
//! 
#![warn(missing_docs)]

pub mod script;
pub mod error;
pub mod runtime;

pub use deno_core;