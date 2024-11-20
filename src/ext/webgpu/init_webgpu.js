import * as init from 'ext:deno_webgpu/00_init.js';
//import * as webgpu from 'ext:deno_webgpu/01_webgpu.js';
import * as webgpuSurface from 'ext:deno_webgpu/02_surface.js';

globalThis.Deno.UnsafeWindowSurface = webgpuSurface.UnsafeWindowSurface;



/*
import { applyToGlobal, nonEnumerable } from 'ext:rustyscript/rustyscript.js';
applyToGlobal({
    GPU: nonEnumerable((webgpu) => webgpu.GPU, loadWebGPU),
    GPUAdapter: nonEnumerable(
        (webgpu) => webgpu.GPUAdapter,
        loadWebGPU,
    ),
    GPUAdapterInfo: nonEnumerable(
        (webgpu) => webgpu.GPUAdapterInfo,
        loadWebGPU,
    ),
    GPUBuffer: nonEnumerable(
        (webgpu) => webgpu.GPUBuffer,
        loadWebGPU,
    ),
    GPUBufferUsage: nonEnumerable(
        (webgpu) => webgpu.GPUBufferUsage,
        loadWebGPU,
    ),
    GPUCanvasContext: nonEnumerable(webgpuSurface.GPUCanvasContext),
    GPUColorWrite: nonEnumerable(
        (webgpu) => webgpu.GPUColorWrite,
        loadWebGPU,
    ),
    GPUCommandBuffer: nonEnumerable(
        (webgpu) => webgpu.GPUCommandBuffer,
        loadWebGPU,
    ),
    GPUCommandEncoder: nonEnumerable(
        (webgpu) => webgpu.GPUCommandEncoder,
        loadWebGPU,
    ),
    GPUComputePassEncoder: nonEnumerable(
        (webgpu) => webgpu.GPUComputePassEncoder,
        loadWebGPU,
    ),
    GPUComputePipeline: nonEnumerable(
        (webgpu) => webgpu.GPUComputePipeline,
        loadWebGPU,
    ),
    GPUDevice: nonEnumerable(
        (webgpu) => webgpu.GPUDevice,
        loadWebGPU,
    ),
    GPUDeviceLostInfo: nonEnumerable(
        (webgpu) => webgpu.GPUDeviceLostInfo,
        loadWebGPU,
    ),
    GPUError: nonEnumerable(
        (webgpu) => webgpu.GPUError,
        loadWebGPU,
    ),
    GPUBindGroup: nonEnumerable(
        (webgpu) => webgpu.GPUBindGroup,
        loadWebGPU,
    ),
    GPUBindGroupLayout: nonEnumerable(
        (webgpu) => webgpu.GPUBindGroupLayout,
        loadWebGPU,
    ),
    GPUInternalError: nonEnumerable(
        (webgpu) => webgpu.GPUInternalError,
        loadWebGPU,
    ),
    GPUPipelineError: nonEnumerable(
        (webgpu) => webgpu.GPUPipelineError,
        loadWebGPU,
    ),
    GPUUncapturedErrorEvent: nonEnumerable(
        (webgpu) => webgpu.GPUUncapturedErrorEvent,
        loadWebGPU,
    ),
    GPUPipelineLayout: nonEnumerable(
        (webgpu) => webgpu.GPUPipelineLayout,
        loadWebGPU,
    ),
    GPUQueue: nonEnumerable(
        (webgpu) => webgpu.GPUQueue,
        loadWebGPU,
    ),
    GPUQuerySet: nonEnumerable(
        (webgpu) => webgpu.GPUQuerySet,
        loadWebGPU,
    ),
    GPUMapMode: nonEnumerable(
        (webgpu) => webgpu.GPUMapMode,
        loadWebGPU,
    ),
    GPUOutOfMemoryError: nonEnumerable(
        (webgpu) => webgpu.GPUOutOfMemoryError,
        loadWebGPU,
    ),
    GPURenderBundle: nonEnumerable(
        (webgpu) => webgpu.GPURenderBundle,
        loadWebGPU,
    ),
    GPURenderBundleEncoder: nonEnumerable(
        (webgpu) => webgpu.GPURenderBundleEncoder,
        loadWebGPU,
    ),
    GPURenderPassEncoder: nonEnumerable(
        (webgpu) => webgpu.GPURenderPassEncoder,
        loadWebGPU,
    ),
    GPURenderPipeline: nonEnumerable(
        (webgpu) => webgpu.GPURenderPipeline,
        loadWebGPU,
    ),
    GPUSampler: nonEnumerable(
        (webgpu) => webgpu.GPUSampler,
        loadWebGPU,
    ),
    GPUShaderModule: nonEnumerable(
        (webgpu) => webgpu.GPUShaderModule,
        loadWebGPU,
    ),
    GPUShaderStage: nonEnumerable(
        (webgpu) => webgpu.GPUShaderStage,
        loadWebGPU,
    ),
    GPUSupportedFeatures: nonEnumerable(
        (webgpu) => webgpu.GPUSupportedFeatures,
        loadWebGPU,
    ),
    GPUSupportedLimits: nonEnumerable(
        (webgpu) => webgpu.GPUSupportedLimits,
        loadWebGPU,
    ),
    GPUTexture: nonEnumerable(
        (webgpu) => webgpu.GPUTexture,
        loadWebGPU,
    ),
    GPUTextureView: nonEnumerable(
        (webgpu) => webgpu.GPUTextureView,
        loadWebGPU,
    ),
    GPUTextureUsage: nonEnumerable(
        (webgpu) => webgpu.GPUTextureUsage,
        loadWebGPU,
    ),
    GPUValidationError: nonEnumerable(
        (webgpu) => webgpu.GPUValidationError,
        loadWebGPU,
    ),
})*/