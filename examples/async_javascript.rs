use rustyscript::{json_args, tokio, Error, Module, Runtime};

fn main() -> Result<(), Error> {
    let module = Module::new(
        "test.js",
        "
        function resolve_after(t) {
            return new Promise((resolve) => {
                setTimeout(() => {
                    console.log('Finished after ' + t + 'ms');
                    resolve('resolved');
                }, t);
            });
        }
        export const f1 = async () => await resolve_after(2000);
        export const f2 = async () => await resolve_after(1000);
        export const f3 = async () => await resolve_after(4000);
        export const f4 = async () => await resolve_after(8000);
    ",
    );

    let mut runtime = Runtime::new(Default::default())?;
    runtime.load_module(&module)?;

    let handle = runtime.load_module(&module)?;

    let f1 = runtime.call_async_function(Some(&handle), "f1", json_args!());
    let f2 = runtime.call_async_function(Some(&handle), "f2", json_args!());
    let f3 = runtime.call_async_function(Some(&handle), "f3", json_args!());
    let f4 = runtime.call_async_function(Some(&handle), "f4", json_args!());

    //
    // Now we await all 4 on different tokio threads
    //

    let f1 = tokio::spawn(async move {
        f1.await;
        println!("f1 done");
    });

    let f2 = tokio::spawn(async move {
        f2.await;
        println!("f2 done");
    });

    let f3 = tokio::spawn(async move {
        f3.await;
        println!("f3 done");
    });

    let f4 = tokio::spawn(async move {
        f4.await;
        println!("f4 done");
    });

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            f1.await.unwrap();
            f2.await.unwrap();
            f3.await.unwrap();
            f4.await.unwrap();
        });

    Ok(())
}
