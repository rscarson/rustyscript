# Update Tool

It will update Cargo.toml files in the parent directory to match specific deno release.

## Usage

```bash
# switch to this directory
cd tools

# Update to the latest version
cargo run --bin update

# Update to a specific version
cargo run --bin update -- 2.2.6
```
