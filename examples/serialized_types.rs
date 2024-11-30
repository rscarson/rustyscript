///
/// This example is meant to demonstrate sending and receiving custom types
/// between JS and rust
///
use rustyscript::{module, Error, Module, ModuleWrapper};
use serde::{Deserialize, Serialize};

// Modules can be defined statically using this macro!
static MY_MODULE: Module = module!(
    "custom_types.js",
    "
    // Mapping the enum types over like this isn't strictly needed
    // But it does help prevent bugs!
    const BridgeCrossingResults = {
        Permitted: 'Permitted',
        Denied: 'Denied'
    };

    const Quests = {
        HolyGrail: 'HolyGrail',
        Groceries: 'Groceries',
        Sandwich: 'Sandwich',
    }

    export function checkAttempt(attempt) {
        if (attempt.quest == Quests.HolyGrail) {
            return BridgeCrossingResults.Permitted;
        } else {
            return BridgeCrossingResults.Denied;
        }
    }
"
);

/// This enum will be used by both rust and JS
/// It will be returned by JS, and thus needs Deserialize
/// The other 2 traits are only for the assert_eq! macro below
#[derive(Deserialize, PartialEq, Debug)]
enum BridgeCrossingResult {
    Permitted,
    Denied,
}

/// This enum will be used by both rust and JS
/// Since it is being send to JS, it needs Serialize
#[derive(Serialize)]
enum Quest {
    HolyGrail,
    Groceries,
}

/// This type will be sent into the JS module
/// Since it is being send to JS, it needs Serialize
#[derive(Serialize)]
struct BridgeCrossingAttempt {
    name: String,
    quest: Quest,
    favourite_colour: String,
}

fn main() -> Result<(), Error> {
    // We only have one source file, so it is simpler here to just use this wrapper type
    // As opposed to building a complete runtime.
    let mut module = ModuleWrapper::new_from_module(&MY_MODULE, Default::default())?;

    // Although we can use json_args!() to call a function with primitives as arguments
    // More complicated types must use `Runtime::arg`
    let result: BridgeCrossingResult = module.call(
        "checkAttempt",
        &BridgeCrossingAttempt {
            name: "Lancelot".to_string(),
            quest: Quest::Groceries,
            favourite_colour: "blue".to_string(),
        },
    )?;
    assert_eq!(result, BridgeCrossingResult::Denied);

    // Let us try again with different values...
    let result: BridgeCrossingResult = module.call(
        "checkAttempt",
        &BridgeCrossingAttempt {
            name: "Lancelot".to_string(),
            quest: Quest::HolyGrail,
            favourite_colour: "blue".to_string(),
        },
    )?;
    assert_eq!(result, BridgeCrossingResult::Permitted);

    Ok(())
}
