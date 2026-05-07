import { core } from 'ext:core/mod.js';
import { loadWebGPU } from 'ext:deno_webgpu/00_init.js';
import { GPUCanvasContext, UnsafeWindowSurface } from 'ext:deno_webgpu/02_surface.js';

const lazy = (name) =>
    core.propNonEnumerableLazyLoaded((webgpu) => webgpu[name], loadWebGPU);

Object.defineProperties(globalThis, {
    GPU: lazy('GPU'),
    GPUAdapter: lazy('GPUAdapter'),
    GPUAdapterInfo: lazy('GPUAdapterInfo'),
    GPUBindGroup: lazy('GPUBindGroup'),
    GPUBindGroupLayout: lazy('GPUBindGroupLayout'),
    GPUBuffer: lazy('GPUBuffer'),
    GPUBufferUsage: lazy('GPUBufferUsage'),
    GPUColorWrite: lazy('GPUColorWrite'),
    GPUCommandBuffer: lazy('GPUCommandBuffer'),
    GPUCommandEncoder: lazy('GPUCommandEncoder'),
    GPUCompilationInfo: lazy('GPUCompilationInfo'),
    GPUCompilationMessage: lazy('GPUCompilationMessage'),
    GPUComputePassEncoder: lazy('GPUComputePassEncoder'),
    GPUComputePipeline: lazy('GPUComputePipeline'),
    GPUDevice: lazy('GPUDevice'),
    GPUDeviceLostInfo: lazy('GPUDeviceLostInfo'),
    GPUError: lazy('GPUError'),
    GPUExternalTexture: lazy('GPUExternalTexture'),
    GPUInternalError: lazy('GPUInternalError'),
    GPUMapMode: lazy('GPUMapMode'),
    GPUOutOfMemoryError: lazy('GPUOutOfMemoryError'),
    GPUPipelineLayout: lazy('GPUPipelineLayout'),
    GPUQuerySet: lazy('GPUQuerySet'),
    GPUQueue: lazy('GPUQueue'),
    GPURenderBundle: lazy('GPURenderBundle'),
    GPURenderBundleEncoder: lazy('GPURenderBundleEncoder'),
    GPURenderPassEncoder: lazy('GPURenderPassEncoder'),
    GPURenderPipeline: lazy('GPURenderPipeline'),
    GPUSampler: lazy('GPUSampler'),
    GPUShaderModule: lazy('GPUShaderModule'),
    GPUShaderStage: lazy('GPUShaderStage'),
    GPUSupportedFeatures: lazy('GPUSupportedFeatures'),
    GPUSupportedLimits: lazy('GPUSupportedLimits'),
    GPUTexture: lazy('GPUTexture'),
    GPUTextureUsage: lazy('GPUTextureUsage'),
    GPUTextureView: lazy('GPUTextureView'),
    GPUUncapturedErrorEvent: lazy('GPUUncapturedErrorEvent'),
    GPUValidationError: lazy('GPUValidationError'),
    GPUCanvasContext: core.propNonEnumerable(GPUCanvasContext),
});

if (typeof globalThis.navigator !== 'object' || globalThis.navigator === null) {
    Object.defineProperty(globalThis, 'navigator', {
        configurable: true,
        enumerable: true,
        writable: true,
        value: {},
    });
}
Object.defineProperty(globalThis.navigator, 'gpu', {
    configurable: true,
    enumerable: true,
    get() {
        const webgpu = loadWebGPU();
        webgpu.initGPU();
        return webgpu.gpu;
    },
});

globalThis.Deno.UnsafeWindowSurface = UnsafeWindowSurface;
