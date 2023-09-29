///
/// This example is meant to demonstrate the basic usage of entrypoint functions
///
/// A module can optionally have an entrypoint function (that can return a value and accept args)
/// which can be called from rust on load.
///
/// The same effect can be achieved by calling a function later, so they are optional
/// They are most useful in the context of Runtime::execute_module, which can be seen
/// in the 'hello_world' example.
///
use js_playground::{Error, ModuleHandle, Runtime, Script};
use std::time::{Duration, Instant};

#[macro_use]
mod error_macro {
    /// Maps one error type to another
    macro_rules! benchmark {
        ($code:block, $now_container:ident, $elapsed_container:ident) => {
            $now_container = Instant::now();
            $code
            $elapsed_container = $now_container.elapsed();
        };
    }
}

fn main() -> Result<(), Error> {
    let mut now: Instant;
    let mut elapsed: Duration;
    let script = Script::new(
        "test.js",
        "
        let i = 0;
        while (++i < 10000) {}
        console.log('done');

        export const getValue = () => i;
        js_playground.register_entrypoint(() => 'bonk');
        ",
    );

    let mut runtime: Runtime;
    let module: ModuleHandle;
    let value: i64;

    benchmark!(
        {
            runtime = Runtime::new(Default::default()).expect("Could not create runtime");
        },
        now,
        elapsed
    );
    println!("Created the runtime in {}ms", elapsed.as_millis());

    benchmark!(
        {
            module = runtime
                .load_modules(&script, vec![])
                .expect("Could not load module");
        },
        now,
        elapsed
    );
    println!("Loaded the module in {}ms", elapsed.as_millis());

    benchmark!(
        {
            value = runtime
                .call_function::<i64>(&module, "getValue", Runtime::EMPTY_ARGS)
                .expect("Could not call function");
        },
        now,
        elapsed
    );
    println!("Called the function in {}ms", elapsed.as_millis());

    benchmark!(
        {
            runtime.reset();
        },
        now,
        elapsed
    );
    println!("Reset the runtime in {}ms", elapsed.as_millis());

    benchmark!(
        {
            runtime
                .load_modules(&script, vec![])
                .expect("Could not load module");
        },
        now,
        elapsed
    );
    println!("Reloaded the module in {}ms", elapsed.as_millis());

    assert_eq!(10000, value);
    Ok(())
}
