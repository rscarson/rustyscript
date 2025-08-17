//!
//! This example shows the `op_whitelist` feature; a security mechanism allowing CI to catch sandbox-breaking OPs before releases
//!
use rustyscript::{op_whitelist::get_whitelist, Error, Runtime, RuntimeOptions};

fn main() -> Result<(), Error> {
    let mut runtime = Runtime::new(RuntimeOptions::default())?;
    let whitelist = get_whitelist();

    let unsafe_ops = whitelist.unsafe_ops(&mut runtime);
    if !unsafe_ops.is_empty() {
        eprintln!("Found {} unsafe ops:", unsafe_ops.len());
        for op in unsafe_ops {
            eprintln!(" - {}", op);
        }

        eprintln!("Please review and stub or whitelist this above!");
        std::process::exit(1);
    }

    println!("No unsafe ops found!");
    Ok(())
}
