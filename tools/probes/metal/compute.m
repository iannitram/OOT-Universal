#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

enum { kNoMetalDevice = 77, kElementCount = 4 };

static const char *kShaderSource =
    "#include <metal_stdlib>\n"
    "using namespace metal;\n"
    "kernel void add_offset(device uint *values [[buffer(0)]],\n"
    "                      uint index [[thread_position_in_grid]]) {\n"
    "    if (index < 4) values[index] += 0x13579bdfu;\n"
    "}\n";

static int failWithNSError(const char *operation, NSError *error) {
    fprintf(stderr, "%s failed: %s\n", operation,
            error.localizedDescription.UTF8String ?: "unknown Metal error");
    return EXIT_FAILURE;
}

int main(void) {
    @autoreleasepool {
        id<MTLDevice> device = MTLCreateSystemDefaultDevice();
        if (device == nil) {
            fprintf(stderr, "NO_METAL_DEVICE: native Metal dispatch unavailable\n");
            return kNoMetalDevice;
        }

        printf("Metal device: %s\n", device.name.UTF8String);

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

        if (commandBuffer.status == MTLCommandBufferStatusError) {
            return failWithNSError("Metal compute dispatch", commandBuffer.error);
        }

        const uint32_t *readback = (const uint32_t *)buffer.contents;
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
        return EXIT_SUCCESS;
    }
}
