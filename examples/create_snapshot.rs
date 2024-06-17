///
/// This example demonstrates how to create a snapshot from a module and save it to a file.
/// Snapshots can be used to massively decrease the startup time of a Runtime instance (15ms -> 3ms) by pre-loading
/// extensions and modules into the runtime state before it is created. A snapshot can be used on any runtime with
/// the same set of extensions and options as the runtime that created it.
///
fn main() -> Result<(), Error> {
    // A module we want pre-loaded into the snapshot
    let module = Module::new(
        "my_module.js",
        "export function importantFunction() { return 42; }",
    );

    // Create a snapshot with default runtime options
    // These options need to be the same as the ones used to create the runtime
    let snapshot = SnapshotBuilder::new(Default::default())?
        .with_module(&module)?
        .finish();

    // Save the snapshot to a file
    fs::write("snapshot.bin", snapshot)?;
    Ok(())

    // To use the snapshot, load it with `include_bytes!` into the `RuntimeOptions` struct:
    // const STARTUP_SNAPSHOT: &[u8] = include_bytes!("snapshot.bin");
    // let options = RuntimeOptions {
    //     startup_snapshot: Some(STARTUP_SNAPSHOT),
    //     ..Default::default()
    // };
}
