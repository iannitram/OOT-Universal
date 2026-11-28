#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

enum { kNoMetalDevice = 77, kElementCount = 4, kRenderWidth = 64, kRenderHeight = 64 };

static const char *kShaderSource =
    "#include <metal_stdlib>\n"
    "using namespace metal;\n"
    "kernel void add_offset(device uint *values [[buffer(0)]],\n"
    "                      uint index [[thread_position_in_grid]]) {\n"
    "    if (index < 4) values[index] += 0x13579bdfu;\n"
    "}\n"
    "struct VertexOut { float4 position [[position]]; };\n"
    "vertex VertexOut triangle_vertex(uint vertexID [[vertex_id]]) {\n"
    "    const float2 positions[3] = { float2(-0.7, -0.7), float2(0.7, -0.7), float2(0.0, 0.7) };\n"
    "    VertexOut result; result.position = float4(positions[vertexID], 0.0, 1.0); return result;\n"
    "}\n"
    "fragment float4 triangle_fragment() { return float4(1.0, 0.0, 0.0, 1.0); }\n";

static int failWithNSError(const char *operation, NSError *error) {
    fprintf(stderr, "%s failed: %s\n", operation,
            error.localizedDescription.UTF8String ?: "unknown Metal error");
    return EXIT_FAILURE;
}

int main(void) {
    @autoreleasepool {
        id<MTLDevice> device = MTLCreateSystemDefaultDevice();
        if (device == nil) {
            fprintf(stderr, "NO_METAL_DEVICE: native Metal compute and render unavailable\n");
            return kNoMetalDevice;
        }

        const char *deviceName = device.name.UTF8String ?: "unknown";
        printf("Metal device: %s\n", deviceName);
        if (strstr(deviceName, "Paravirtual") != NULL || strstr(deviceName, "paravirtual") != NULL) {
            printf("Metal device class: paravirtual (no physical-GPU claim)\n");
        } else {
            printf("Metal device class: unclassified (physical hardware not verified)\n");
        }

        NSError *error = nil;
        id<MTLLibrary> library = [device newLibraryWithSource:
            [NSString stringWithUTF8String:kShaderSource]
            options:nil error:&error];
        if (library == nil) return failWithNSError("Metal shader compilation", error);

        id<MTLFunction> function = [library newFunctionWithName:@"add_offset"];
        if (function == nil) {
            fprintf(stderr, "Metal shader function add_offset was not found\n");
            return EXIT_FAILURE;
        }

        id<MTLComputePipelineState> pipeline = [device newComputePipelineStateWithFunction:function error:&error];
        if (pipeline == nil) return failWithNSError("Metal pipeline creation", error);

        id<MTLCommandQueue> queue = [device newCommandQueue];
        if (queue == nil) {
            fprintf(stderr, "Metal command queue creation failed\n");
            return EXIT_FAILURE;
        }

        uint32_t values[kElementCount] = {0, 1, 2, 3};
        id<MTLBuffer> buffer = [device newBufferWithBytes:values
                                                   length:sizeof(values)
                                                  options:MTLResourceStorageModeShared];
        if (buffer == nil) {
            fprintf(stderr, "Shared Metal buffer allocation failed\n");
            return EXIT_FAILURE;
        }

        id<MTLCommandBuffer> commandBuffer = [queue commandBuffer];
        id<MTLComputeCommandEncoder> encoder = [commandBuffer computeCommandEncoder];
        if (commandBuffer == nil || encoder == nil) {
            fprintf(stderr, "Metal command buffer or compute encoder creation failed\n");
            return EXIT_FAILURE;
        }

        [encoder setComputePipelineState:pipeline];
        [encoder setBuffer:buffer offset:0 atIndex:0];
        [encoder dispatchThreads:MTLSizeMake(kElementCount, 1, 1)
            threadsPerThreadgroup:MTLSizeMake(kElementCount, 1, 1)];
        [encoder endEncoding];
        [commandBuffer commit];
        [commandBuffer waitUntilCompleted];

        if (commandBuffer.status != MTLCommandBufferStatusCompleted) {
            return failWithNSError("Metal compute dispatch", commandBuffer.error);
        }

        const uint32_t *readback = (const uint32_t *)buffer.contents;
        if (readback == NULL) {
            fprintf(stderr, "Shared Metal buffer has no host address\n");
            return EXIT_FAILURE;
        }
        for (uint32_t i = 0; i < kElementCount; ++i) {
            const uint32_t expected = i + 0x13579bdfu;
            if (readback[i] != expected) {
                fprintf(stderr, "Readback mismatch at %u: got 0x%08x, expected 0x%08x\n",
                        i, readback[i], expected);
                return EXIT_FAILURE;
            }
        }

        printf("PASS: native Metal compute and shared-buffer readback verified (0x%08x..0x%08x)\n",
               readback[0], readback[kElementCount - 1]);

        id<MTLFunction> vertexFunction = [library newFunctionWithName:@"triangle_vertex"];
        id<MTLFunction> fragmentFunction = [library newFunctionWithName:@"triangle_fragment"];
        if (vertexFunction == nil || fragmentFunction == nil) {
            fprintf(stderr, "Metal triangle vertex or fragment function was not found\n");
            return EXIT_FAILURE;
        }

        MTLRenderPipelineDescriptor *renderDescriptor = [[MTLRenderPipelineDescriptor alloc] init];
        renderDescriptor.vertexFunction = vertexFunction;
        renderDescriptor.fragmentFunction = fragmentFunction;
        renderDescriptor.colorAttachments[0].pixelFormat = MTLPixelFormatRGBA8Unorm;
        id<MTLRenderPipelineState> renderPipeline =
            [device newRenderPipelineStateWithDescriptor:renderDescriptor error:&error];
        if (renderPipeline == nil) return failWithNSError("Metal render pipeline creation", error);

        MTLTextureDescriptor *textureDescriptor = [MTLTextureDescriptor
            texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
                                         width:kRenderWidth height:kRenderHeight mipmapped:NO];
        textureDescriptor.usage = MTLTextureUsageRenderTarget;
        textureDescriptor.storageMode = MTLStorageModePrivate;
        id<MTLTexture> texture = [device newTextureWithDescriptor:textureDescriptor];
        if (texture == nil) {
            fprintf(stderr, "Private Metal render target allocation failed\n");
            return EXIT_FAILURE;
        }

        const NSUInteger bytesPerRow = kRenderWidth * 4;
        id<MTLBuffer> pixels = [device newBufferWithLength:bytesPerRow * kRenderHeight
                                                   options:MTLResourceStorageModeShared];
        if (pixels == nil) {
            fprintf(stderr, "Shared Metal pixel readback allocation failed\n");
            return EXIT_FAILURE;
        }

        MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
        pass.colorAttachments[0].texture = texture;
        pass.colorAttachments[0].loadAction = MTLLoadActionClear;
        pass.colorAttachments[0].storeAction = MTLStoreActionStore;
        pass.colorAttachments[0].clearColor = MTLClearColorMake(0.0, 0.0, 0.0, 1.0);

        id<MTLCommandBuffer> renderCommandBuffer = [queue commandBuffer];
        if (renderCommandBuffer == nil) {
            fprintf(stderr, "Metal render command buffer creation failed\n");
            return EXIT_FAILURE;
        }
        id<MTLRenderCommandEncoder> renderEncoder = [renderCommandBuffer renderCommandEncoderWithDescriptor:pass];
        if (renderEncoder == nil) {
            fprintf(stderr, "Metal render encoder creation failed\n");
            return EXIT_FAILURE;
        }
        [renderEncoder setRenderPipelineState:renderPipeline];
        [renderEncoder drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:3];
        [renderEncoder endEncoding];

        id<MTLBlitCommandEncoder> blitEncoder = [renderCommandBuffer blitCommandEncoder];
        if (blitEncoder == nil) {
            fprintf(stderr, "Metal pixel readback blit encoder creation failed\n");
            return EXIT_FAILURE;
        }
        [blitEncoder copyFromTexture:texture sourceSlice:0 sourceLevel:0
                         sourceOrigin:MTLOriginMake(0, 0, 0)
                           sourceSize:MTLSizeMake(kRenderWidth, kRenderHeight, 1)
                             toBuffer:pixels destinationOffset:0 destinationBytesPerRow:bytesPerRow
                 destinationBytesPerImage:bytesPerRow * kRenderHeight];
        [blitEncoder endEncoding];
        [renderCommandBuffer commit];
        [renderCommandBuffer waitUntilCompleted];
        if (renderCommandBuffer.status != MTLCommandBufferStatusCompleted) {
            return failWithNSError("Metal triangle render and readback", renderCommandBuffer.error);
        }

        const uint8_t *pixelBytes = (const uint8_t *)pixels.contents;
        if (pixelBytes == NULL) {
            fprintf(stderr, "Shared Metal pixel buffer has no host address\n");
            return EXIT_FAILURE;
        }
        const uint8_t *center = pixelBytes + ((kRenderHeight / 2) * bytesPerRow) + ((kRenderWidth / 2) * 4);
        const uint8_t *corner = pixelBytes;
        if (center[0] < 250 || center[1] > 5 || center[2] > 5 || center[3] < 250) {
            fprintf(stderr, "Triangle center pixel mismatch: got RGBA(%u,%u,%u,%u), expected red\n",
                    center[0], center[1], center[2], center[3]);
            return EXIT_FAILURE;
        }
        if (corner[0] > 5 || corner[1] > 5 || corner[2] > 5 || corner[3] < 250) {
            fprintf(stderr, "Triangle corner pixel mismatch: got RGBA(%u,%u,%u,%u), expected black\n",
                    corner[0], corner[1], corner[2], corner[3]);
            return EXIT_FAILURE;
        }
        printf("PASS: native Metal offscreen triangle render and blit readback verified (center red, corner black)\n");
        return EXIT_SUCCESS;
    }
}
