use std::time::Duration;

use deno_core::PollEventLoopOptions;
///
/// This example the use of async module loading, and the handing of ongoing
/// background tasks.
///
use rustyscript::{Error, Module, ModuleHandle, Runtime, RuntimeOptions};

fn main() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        "
        // A basic messaging queue
        const messages = [];
        export function nextMessage() {
            if (messages.length === 0) {
                return '';
            }
            return messages.shift();
        }

        const socket = new WebSocket('wss://echo.websocket.org');
        export function sendMessage(text) {
            socket.send(text);
        }

        socket.addEventListener('error', (e) => {
            clearInterval(t);
            throw(e);
        });

        socket.addEventListener('open', (event) => {
            console.log('Open');
            socket.send('Socket Open!');

            setTimeout(() => {
                // Send a message after 5 seconds
                sendMessage('ping');
            }, 5000);

            setTimeout(() => {
                // Send a message after 10 seconds
                sendMessage('pong');
            }, 10000);

            setTimeout(() => {
                // Close the socket after 15 seconds
                socket.close();

                // Clear the interval, ending the event loop
                console.log('Closing socket');
                clearInterval(t);
            }, 15000);
        });

        socket.addEventListener('message', (event) => {
            console.log('Received a message');
            messages.push(event.data);
        });

        // Keep the event loop alive
        let t = setInterval(() => {
        }, 1000);
    ",
    );

    // Whitelist the echo server for certificate errors
    let mut options = RuntimeOptions::default();
    options
        .extension_options
        .web
        .whitelist_certificate_for("echo.websocket.org");

    let mut runtime = Runtime::new(options)?;
    let tokio_runtime = runtime.tokio_runtime();

    // Load the module
    // This will run the event loop until the module is fully loaded, or an error occurs
    let module_handle = tokio_runtime.block_on(runtime.load_module_async(&module))?;

    // Run the event loop until it reports that it has finished
    while runtime.advance_event_loop(PollEventLoopOptions::default())? {
        // Check for messages from the module
        if let Some(msg) = check_for_messages(&mut runtime, &module_handle)? {
            println!("Received message: {}", msg);
        }

        // Run the event loop for 50ms
        runtime.block_on_event_loop(
            PollEventLoopOptions::default(),
            Some(Duration::from_millis(50)),
        )?;
    }

    Ok(())
}

fn check_for_messages(
    rt: &mut Runtime,
    module_handle: &ModuleHandle,
) -> Result<Option<String>, Error> {
    let next_message: String =
        rt.call_function_immediate(Some(module_handle), "nextMessage", &())?;
    if next_message.is_empty() {
        Ok(None)
    } else {
        Ok(Some(next_message))
    }
}
