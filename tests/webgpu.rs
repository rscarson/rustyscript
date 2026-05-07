//! Regression tests for issue #416 — WebGPU initialization.
//!
//! Before the fix, only `Deno.UnsafeWindowSurface` was wired up; every other
//! `GPU*` global was commented out in `init_webgpu.js`. These tests verify
//! that the standard WebGPU surface (constructors and `navigator.gpu`) is
//! reachable from JavaScript when the `webgpu` feature is enabled.

#![cfg(feature = "webgpu")]

use rustyscript::{json_args, serde_json, Module, Runtime, RuntimeOptions};

fn run(src: &str) -> Result<String, rustyscript::Error> {
    let module = Module::new("webgpu_test.js", src);
    let mut runtime = Runtime::new(RuntimeOptions::default())?;
    let handle = runtime.load_module(&module)?;
    runtime.call_entrypoint(&handle, json_args!())
}

#[test]
fn webgpu_globals_are_defined() {
    let names = [
        "GPU",
        "GPUAdapter",
        "GPUAdapterInfo",
        "GPUBindGroup",
        "GPUBindGroupLayout",
        "GPUBuffer",
        "GPUBufferUsage",
        "GPUColorWrite",
        "GPUCommandBuffer",
        "GPUCommandEncoder",
        "GPUCompilationInfo",
        "GPUCompilationMessage",
        "GPUComputePassEncoder",
        "GPUComputePipeline",
        "GPUDevice",
        "GPUDeviceLostInfo",
        "GPUError",
        "GPUExternalTexture",
        "GPUInternalError",
        "GPUMapMode",
        "GPUOutOfMemoryError",
        "GPUPipelineLayout",
        "GPUQuerySet",
        "GPUQueue",
        "GPURenderBundle",
        "GPURenderBundleEncoder",
        "GPURenderPassEncoder",
        "GPURenderPipeline",
        "GPUSampler",
        "GPUShaderModule",
        "GPUShaderStage",
        "GPUSupportedFeatures",
        "GPUSupportedLimits",
        "GPUTexture",
        "GPUTextureUsage",
        "GPUTextureView",
        "GPUUncapturedErrorEvent",
        "GPUValidationError",
        "GPUCanvasContext",
    ];

    let names_js = names
        .iter()
        .map(|n| format!("\"{n}\""))
        .collect::<Vec<_>>()
        .join(",");

    let src = format!(
        r#"
        const expected = [{names_js}];
        const missing = expected.filter(n => typeof globalThis[n] !== "function");
        export default () => {{
            if (missing.length > 0) throw new Error("missing: " + missing.join(","));
            return "ok";
        }};
        "#
    );

    let result = run(&src).expect("module should evaluate");
    assert_eq!(result, "ok");
}

#[test]
fn navigator_gpu_is_exposed() {
    let result = run(
        r#"
        export default () => {
            if (typeof navigator !== "object" || navigator === null) {
                throw new Error("navigator is not an object");
            }
            if (typeof navigator.gpu !== "object" || navigator.gpu === null) {
                throw new Error("navigator.gpu is not an object");
            }
            if (typeof navigator.gpu.requestAdapter !== "function") {
                throw new Error("navigator.gpu.requestAdapter missing");
            }
            return "ok";
        };
        "#,
    )
    .expect("module should evaluate");
    assert_eq!(result, "ok");
}

#[test]
fn unsafe_window_surface_is_exposed() {
    let result = run(
        r#"
        export default () => {
            if (typeof Deno.UnsafeWindowSurface !== "function") {
                throw new Error("Deno.UnsafeWindowSurface missing");
            }
            return "ok";
        };
        "#,
    )
    .expect("module should evaluate");
    assert_eq!(result, "ok");
}

#[test]
fn gpu_buffer_usage_constants() {
    let result = run(
        r#"
        export default () => {
            if (GPUBufferUsage.MAP_READ !== 0x0001) throw new Error("MAP_READ");
            if (GPUBufferUsage.MAP_WRITE !== 0x0002) throw new Error("MAP_WRITE");
            if (GPUBufferUsage.COPY_SRC !== 0x0004) throw new Error("COPY_SRC");
            if (GPUBufferUsage.STORAGE !== 0x0080) throw new Error("STORAGE");
            return "ok";
        };
        "#,
    )
    .expect("module should evaluate");
    assert_eq!(result, "ok");
}

/// Run a real WGSL compute shader that doubles every element of an input
/// array, and verify the GPU-produced results match the CPU expectation.
///
/// If no GPU adapter is available (e.g. CI without graphics drivers), the
/// test prints a notice and returns success rather than failing — the goal
/// is to verify the WebGPU pipeline works when hardware is present.
#[test]
fn gpu_compute_doubles_values() {
    let module = Module::new(
        "gpu_compute.js",
        r#"
        export default async () => {
            const adapter = await navigator.gpu.requestAdapter();
            if (!adapter) return "no-adapter";

            const device = await adapter.requestDevice();
            const N = 64;
            const input = new Float32Array(N);
            for (let i = 0; i < N; i++) input[i] = i + 1;

            const byteSize = input.byteLength;

            const inputBuffer = device.createBuffer({
                size: byteSize,
                usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
            });
            device.queue.writeBuffer(inputBuffer, 0, input);

            const outputBuffer = device.createBuffer({
                size: byteSize,
                usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
            });

            const readBuffer = device.createBuffer({
                size: byteSize,
                usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
            });

            const shader = device.createShaderModule({
                code: `
                    @group(0) @binding(0) var<storage, read> src: array<f32>;
                    @group(0) @binding(1) var<storage, read_write> dst: array<f32>;
                    @compute @workgroup_size(64)
                    fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
                        let i = gid.x;
                        if (i < arrayLength(&src)) {
                            dst[i] = src[i] * 2.0;
                        }
                    }
                `,
            });

            const pipeline = device.createComputePipeline({
                layout: "auto",
                compute: { module: shader, entryPoint: "main" },
            });

            const bindGroup = device.createBindGroup({
                layout: pipeline.getBindGroupLayout(0),
                entries: [
                    { binding: 0, resource: { buffer: inputBuffer } },
                    { binding: 1, resource: { buffer: outputBuffer } },
                ],
            });

            const encoder = device.createCommandEncoder();
            const pass = encoder.beginComputePass();
            pass.setPipeline(pipeline);
            pass.setBindGroup(0, bindGroup);
            pass.dispatchWorkgroups(Math.ceil(N / 64));
            pass.end();
            encoder.copyBufferToBuffer(outputBuffer, 0, readBuffer, 0, byteSize);
            device.queue.submit([encoder.finish()]);

            await readBuffer.mapAsync(GPUMapMode.READ);
            const result = new Float32Array(readBuffer.getMappedRange().slice(0));
            readBuffer.unmap();

            for (let i = 0; i < N; i++) {
                const expected = (i + 1) * 2;
                if (result[i] !== expected) {
                    throw new Error(`mismatch at ${i}: got ${result[i]}, expected ${expected}`);
                }
            }
            return "ok";
        };
        "#,
    );

    let mut runtime = Runtime::new(RuntimeOptions::default()).expect("runtime");
    let handle = runtime.load_module(&module).expect("load");
    let result: String = runtime
        .call_entrypoint(&handle, json_args!())
        .expect("compute entrypoint");

    if result == "no-adapter" {
        eprintln!("skipping gpu_compute_doubles_values: no GPU adapter available");
        return;
    }
    assert_eq!(result, "ok");
}

// ---------------------------------------------------------------------------
// Diagnostic tests for issue: `device.createComputePipelineAsync(desc)` fails
// with `TypeError: ... 'compute' is required` even though the synchronous
// `device.createComputePipeline(desc)` works with the *same* descriptor.
//
// Root cause confirmed by these tests + reading upstream code:
//   1. `deno_ops-0.256.0/op2/mod.rs:283-287` computes the OpDecl's `arg_count`
//      as `required` when `#[required(N)]` is set, ignoring `is_async`. For
//      `#[async_method] #[required(1)] #[cppgc] async fn create_compute_pipeline_async`
//      (deno_webgpu-0.195.0/device.rs:525) it produces `arg_count = 1` instead
//      of `2` (which the non-`#[required]` branch would correctly compute as
//      `args.len() + is_async as usize`).
//   2. `deno_core-0.380.1/runtime/bindings.rs:599` plumbs that into the v8
//      FunctionTemplate via `.length(arg_count as i32)`, so the raw op exposed
//      to JS reports `Function.prototype.length === 1`.
//   3. `deno_core-0.380.1/00_infra.js:201` (`setUpAsyncStub`) switches on
//      `originalOp.length - 1` to pick an arity-specific stub. With length 1
//      it picks `async_op_0()` which calls `originalOp.call(this, id)` — the
//      user's descriptor is never forwarded, so the WebIDL dictionary
//      converter on the Rust side sees `undefined` and reports `'compute' is
//      required` (the first sorted required field).
//
// Synchronous `createComputePipeline` is fine because no stub is interposed —
// the template's `.length` is informational and the slow_fn reads `args[0]`
// directly.
//
// All `#[async_method] #[required(N)] #[cppgc]` ops in deno_webgpu (and any
// other crate using this combo) are affected. The fix belongs in deno_ops.
// ---------------------------------------------------------------------------

/// Shared JS prelude that obtains a `device` and a valid pipeline `descriptor`,
/// then runs the provided body. The body may use `device`, `descriptor`,
/// `report` (a Map-like object) and must return a result.
///
/// If no GPU adapter is available the body is not run; the entrypoint
/// returns `{ skipped: true }`.
fn run_with_device(body: &str) -> serde_json::Value {
    let module_source = format!(
        r#"
        export default async () => {{
            const adapter = await navigator.gpu.requestAdapter();
            if (!adapter) return {{ skipped: true }};
            const device = await adapter.requestDevice();

            const shader = device.createShaderModule({{
                code: `@compute @workgroup_size(1) fn main() {{}}`,
            }});
            const descriptor = {{
                layout: "auto",
                compute: {{ module: shader, entryPoint: "main" }},
            }};

            const report = {{}};
            {body}
            return report;
        }};
        "#,
    );

    let module = Module::new("webgpu_dispatch_probe.js", &module_source);
    let mut runtime = Runtime::new(RuntimeOptions::default()).expect("runtime");
    let handle = runtime.load_module(&module).expect("load");
    runtime
        .call_entrypoint(&handle, json_args!())
        .expect("dispatch probe entrypoint")
}

/// Probe whether `device.createComputePipelineAsync` is the wrapped
/// `async_op_N(a) { originalOp.call(this, id, a); }` stub installed by
/// `Deno.core.setUpAsyncStub`, or the raw op (which would take the
/// descriptor as `args[0]` and break WebIDL conversion).
///
/// The stub is identifiable by:
///   - `.length === 1` (one declared user argument)
///   - source text containing `originalOp` or the promise_id slot
///
/// A raw op typically has a different `.length` (deno_ops generates the
/// fast/slow fns differently) and its source mentions `op_` symbols.
#[test]
fn compute_pipeline_async_is_wrapped_stub() {
    let report = run_with_device(
        r#"
        const fn = device.createComputePipelineAsync;
        report.typeofFn = typeof fn;
        report.length = fn.length;
        report.name = fn.name;
        report.source = fn.toString();
        // Mentions originalOp / promise id slot? deno_core's setUpAsyncStub
        // produces something like: function async_op_N(a) { return originalOp.call(this, ++p, a); }
        report.looksLikeStub = /originalOp|setUpAsyncStub|\bp\+\+|\+\+p/.test(fn.toString());
        "#,
    );

    eprintln!(
        "createComputePipelineAsync probe: {}",
        serde_json::to_string_pretty(&report).unwrap()
    );

    if report.get("skipped").and_then(|v| v.as_bool()).unwrap_or(false) {
        eprintln!("skipping compute_pipeline_async_is_wrapped_stub: no GPU adapter available");
        return;
    }

    assert_eq!(report["typeofFn"], "function");
    let length = report["length"].as_u64().expect("length must be a number");
    let source = report["source"].as_str().expect("source must be string");
    let looks_like_stub = report["looksLikeStub"].as_bool().unwrap_or(false);

    assert!(
        length == 1 && looks_like_stub,
        "expected createComputePipelineAsync to be the async_op_1 wrapper stub \
         (length=1, source mentions originalOp/promise_id), got length={length}, \
         source={source:?}"
    );
}

/// Call `device.createComputePipelineAsync` directly with an object literal.
///
/// If the async dispatch path is broken (raw op exposed instead of the
/// wrapped stub), the WebIDL converter sees `undefined` at `args[1]` and
/// reports `'compute' is required`. A working dispatch returns a
/// `GPUComputePipeline`.
#[test]
fn compute_pipeline_async_direct_call() {
    let report = run_with_device(
        r#"
        try {
            const pipeline = await device.createComputePipelineAsync(descriptor);
            report.ok = true;
            report.kind = pipeline?.constructor?.name ?? null;
        } catch (e) {
            report.ok = false;
            report.error = String(e);
        }
        "#,
    );

    eprintln!(
        "direct async call: {}",
        serde_json::to_string_pretty(&report).unwrap()
    );

    if report.get("skipped").and_then(|v| v.as_bool()).unwrap_or(false) {
        eprintln!("skipping compute_pipeline_async_direct_call: no GPU adapter available");
        return;
    }

    assert_eq!(
        report["ok"], true,
        "device.createComputePipelineAsync(desc) failed: {}",
        report.get("error").and_then(|v| v.as_str()).unwrap_or("?")
    );
    assert_eq!(report["kind"], "GPUComputePipeline");
}

/// Same call as above but invoked via `Reflect.get(device, name).call(device, desc)`,
/// which is the path wasm-bindgen takes. Some bound-method tricks survive a
/// direct property access but fail through Reflect.get + .call; isolate that.
#[test]
fn compute_pipeline_async_reflect_get_call() {
    let report = run_with_device(
        r#"
        try {
            const fn = Reflect.get(device, 'createComputePipelineAsync');
            report.fnTypeof = typeof fn;
            report.fnLength = fn.length;
            const pipeline = await fn.call(device, descriptor);
            report.ok = true;
            report.kind = pipeline?.constructor?.name ?? null;
        } catch (e) {
            report.ok = false;
            report.error = String(e);
        }
        "#,
    );

    eprintln!(
        "Reflect.get + .call: {}",
        serde_json::to_string_pretty(&report).unwrap()
    );

    if report.get("skipped").and_then(|v| v.as_bool()).unwrap_or(false) {
        eprintln!("skipping compute_pipeline_async_reflect_get_call: no GPU adapter available");
        return;
    }

    assert_eq!(
        report["ok"], true,
        "Reflect.get(device,'createComputePipelineAsync').call(device, desc) failed: {}",
        report.get("error").and_then(|v| v.as_str()).unwrap_or("?")
    );
    assert_eq!(report["kind"], "GPUComputePipeline");
}

/// Sanity check: the *synchronous* `device.createComputePipeline` path with
/// the same descriptor should always succeed when an adapter is available.
/// If sync works but async fails, the bug is isolated to the async dispatch
/// wrapper.
#[test]
fn compute_pipeline_sync_succeeds_with_same_descriptor() {
    let report = run_with_device(
        r#"
        try {
            const pipeline = device.createComputePipeline(descriptor);
            report.ok = true;
            report.kind = pipeline?.constructor?.name ?? null;
        } catch (e) {
            report.ok = false;
            report.error = String(e);
        }
        "#,
    );

    eprintln!(
        "sync createComputePipeline: {}",
        serde_json::to_string_pretty(&report).unwrap()
    );

    if report.get("skipped").and_then(|v| v.as_bool()).unwrap_or(false) {
        eprintln!("skipping compute_pipeline_sync_succeeds_with_same_descriptor: no GPU adapter available");
        return;
    }

    assert_eq!(
        report["ok"], true,
        "device.createComputePipeline(desc) failed: {}",
        report.get("error").and_then(|v| v.as_str()).unwrap_or("?")
    );
    assert_eq!(report["kind"], "GPUComputePipeline");
}
