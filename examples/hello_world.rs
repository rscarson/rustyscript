
use js_playground::{Runtime, Script, Error};

fn main() -> Result<(), Error> {
    let script = Script::new(
        "test.js",
        "js_playground.register_entrypoint(
            () => 2
        )"
    );
    let value: usize = Runtime::execute_module(script, vec![], Default::default())?;
    assert_eq!(value, 2);
    Ok(())
}