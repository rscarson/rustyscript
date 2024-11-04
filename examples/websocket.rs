///
/// This example demonstrates a use for the websockets extension.
/// It will open a connection to the echo server at wss://echo.websocket.org
/// Send a message 'ping', wait for a response, and then close the connection.
///
use rustyscript::{json_args, Error, Module, Runtime, RuntimeOptions, Undefined};

fn main() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        "
        export async function connect(url) {
            return new Promise((resolve, reject) => {
                const ws = new WebSocket(url);

                ws.addEventListener('open', () => {
                    console.log('WebSocket connection opened');
                    console.log(`Sending message 'ping'`);
                    ws.send('ping');
                });

                ws.addEventListener('message', (event) => {
                    console.log(`Received '${event.data}'`);
                    ws.close();
                });

                ws.addEventListener('close', (event) => {
                    if (event.wasClean) {
                        console.log(`Connection closed, code=${event.code} reason='${event.reason}'`);
                        resolve();
                    } else {
                        console.log('Connection died');
                        reject(new Error('Connection died'));
                    }
                });

                ws.addEventListener('error', (e) => {
                    console.log(`Error: ${e}`);
                    reject(new Error(`Error: ${e}`));
                });
            });
        }
    ",
    );

    let mut runtime = Runtime::new(RuntimeOptions {
        default_entrypoint: Some("connect".to_string()),
        ..Default::default()
    })?;

    let module_handle = runtime.load_module(&module)?;

    runtime.call_entrypoint::<Undefined>(&module_handle, json_args!("wss://echo.websocket.org"))?;

    Ok(())
}
