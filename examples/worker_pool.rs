//!
//! This example demonstrates how to use the worker pool to run multiple workers in parallel.
//!
use rustyscript::{
    worker::{DefaultWorker, DefaultWorkerQuery, WorkerPool},
    Error, Module,
};

fn main() -> Result<(), Error> {
    // We first will create a worker pool with 4 workers
    let mut pool = WorkerPool::<DefaultWorker>::new(Default::default(), 4)?;

    // We will now create a module that will perform a long running operation
    // We will run this in parallel on two workers to demonstrate the worker pool
    let module = Module::new(
        "test.js",
        "
        // Perform a long running operation
        for (let i = 0; i < 10000000000; i++) {
            // Do nothing
        }
    ",
    );

    //
    // Start the operation on the first worker
    println!("Start load on A...");
    let worker_a = pool.next_worker();
    let query = DefaultWorkerQuery::LoadModule(module.clone());
    worker_a.borrow().send(query)?; // We don't need to wait for the response right away!

    //
    // Start the operation on the second worker
    println!("Start load on B...");
    let worker_b = pool.next_worker();
    let query = DefaultWorkerQuery::LoadModule(module.clone());
    worker_b.borrow().send(query)?; // We don't need to wait for the response here either

    //
    // We can now wait for the responses
    print!("Waiting for the workers to finish... ");
    worker_a.borrow().receive()?;
    print!("Done A... ");
    worker_b.borrow().receive()?;
    println!("Done B!");

    Ok(())
}
