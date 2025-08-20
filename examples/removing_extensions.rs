//!
//! This example demonstrates how to remove extensions from the extension list.
//!
//! In this way, you can have runtimes in one binary with varying feature sets.
//!
use rustyscript::{Error, RuntimeOptions};

fn main() -> Result<(), Error> {
    // Start with the default feature-set, as defined by Cargo.toml
    let mut options = RuntimeOptions::default();

    println!("Initial extensions:");
    for ext in &options.extensions {
        println!("  - {}", ext.name);
    }

    // Let's unload the console extension
    let removed = options.extensions.unload("console");

    println!("Removed extensions:");
    for ext in removed {
        println!("  - {}", ext.name);
    }

    Ok(())
}
