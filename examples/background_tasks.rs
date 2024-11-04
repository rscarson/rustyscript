use std::time::Duration;

use deno_core::PollEventLoopOptions;
///
/// This example demonstrates a use for the websockets extension.
/// It will open a connection to the echo server at wss://echo.websocket.org
/// Send a message 'ping', wait for a response, and then close the connection.
///
use rustyscript::{Error, Module, ModuleHandle, Runtime, RuntimeOptions};

fn main() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        "
        const messages = [];
        export function nextMessage() {
            if (messages.length === 0) {
                return '';
            }
            return messages.shift();
        }

        export function sendMessage(text) {
            socket.send(text);
        }

        const socket = new WebSocket('wss://echo.websocket.org');

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

    let mut runtime = Runtime::new(RuntimeOptions::default())?;
    let tokio_runtime = runtime.tokio_runtime();

    let module_handle = tokio_runtime.block_on(runtime.load_module_async(&module))?;

    while runtime.advance_event_loop(PollEventLoopOptions::default())? {
        // Check for messages every 50ms
        if let Some(msg) = check_for_messages(&mut runtime, &module_handle)? {
            println!("Received message: {}", msg);
        }

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
