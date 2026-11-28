#include <vulkan/vulkan.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define WIDTH 64u
#define HEIGHT 64u
#define PIXEL_BYTES 4u

static int vk_ok(const char *where, VkResult result) {
    if (result == VK_SUCCESS) return 1;
    fprintf(stderr, "%s failed with VkResult %d\n", where, result);
    return 0;
}

#define CHECK(call) do { if (!vk_ok(#call, (call))) goto cleanup; } while (0)

static unsigned char *read_spv(const char *path, size_t *size_out) {
    FILE *file = fopen(path, "rb");
    if (!file) { perror(path); return NULL; }
    if (fseek(file, 0, SEEK_END) != 0) { fclose(file); return NULL; }
    long length = ftell(file);
    if (length <= 0 || (length % 4) != 0 || fseek(file, 0, SEEK_SET) != 0) {
        fprintf(stderr, "invalid SPIR-V file length: %s\n", path); fclose(file); return NULL;
    }
    unsigned char *bytes = malloc((size_t)length);
    if (!bytes) { fclose(file); return NULL; }
    size_t got = fread(bytes, 1, (size_t)length, file);
    int close_result = fclose(file);
    if (got != (size_t)length || close_result != 0) { free(bytes); return NULL; }
    *size_out = (size_t)length;
    return bytes;
}

static uint32_t find_memory_type(VkPhysicalDevice physical, uint32_t allowed,
                                 VkMemoryPropertyFlags required) {
    VkPhysicalDeviceMemoryProperties props;
    vkGetPhysicalDeviceMemoryProperties(physical, &props);
    for (uint32_t i = 0; i < props.memoryTypeCount; ++i) {
        if ((allowed & (1u << i)) != 0 &&
            (props.memoryTypes[i].propertyFlags & required) == required) return i;
    }
    return UINT32_MAX;
}

int main(int argc, char **argv) {
    if (argc != 3) { fprintf(stderr, "usage: %s <vertex.spv> <fragment.spv>\n", argv[0]); return EXIT_FAILURE; }
    size_t vert_size = 0, frag_size = 0;
    unsigned char *vert_bytes = read_spv(argv[1], &vert_size);
    unsigned char *frag_bytes = read_spv(argv[2], &frag_size);
    if (!vert_bytes || !frag_bytes) { free(vert_bytes); free(frag_bytes); return EXIT_FAILURE; }

    int status = EXIT_FAILURE, queue_idle = 0;
    VkInstance instance = VK_NULL_HANDLE;
    VkDevice device = VK_NULL_HANDLE;
    VkPhysicalDevice physical = VK_NULL_HANDLE;
    VkPhysicalDevice *devices = NULL;
    VkBuffer staging = VK_NULL_HANDLE;
    VkDeviceMemory staging_memory = VK_NULL_HANDLE, image_memory = VK_NULL_HANDLE;
    VkImage image = VK_NULL_HANDLE;
    VkImageView view = VK_NULL_HANDLE;
    VkShaderModule vert = VK_NULL_HANDLE, frag = VK_NULL_HANDLE;
    VkRenderPass render_pass = VK_NULL_HANDLE;
    VkFramebuffer framebuffer = VK_NULL_HANDLE;
    VkPipelineLayout pipeline_layout = VK_NULL_HANDLE;
    VkPipeline pipeline = VK_NULL_HANDLE;
    VkCommandPool command_pool = VK_NULL_HANDLE;

    VkApplicationInfo app = { .sType = VK_STRUCTURE_TYPE_APPLICATION_INFO,
        .pApplicationName = "synthetic-vulkan-offscreen-draw-probe", .apiVersion = VK_API_VERSION_1_0 };
    VkInstanceCreateInfo ici = { .sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO, .pApplicationInfo = &app };
    CHECK(vkCreateInstance(&ici, NULL, &instance));
    uint32_t physical_count = 0;
    CHECK(vkEnumeratePhysicalDevices(instance, &physical_count, NULL));
    devices = calloc(physical_count ? physical_count : 1, sizeof(*devices));
    if (!devices) goto cleanup;
    CHECK(vkEnumeratePhysicalDevices(instance, &physical_count, devices));
    uint32_t family = UINT32_MAX;
    VkPhysicalDeviceProperties selected = {0};
    for (uint32_t i = 0; i < physical_count && physical == VK_NULL_HANDLE; ++i) {
        VkPhysicalDeviceProperties p; vkGetPhysicalDeviceProperties(devices[i], &p);
        if (p.deviceType != VK_PHYSICAL_DEVICE_TYPE_CPU) continue;
        uint32_t count = 0; vkGetPhysicalDeviceQueueFamilyProperties(devices[i], &count, NULL);
        VkQueueFamilyProperties *families = calloc(count ? count : 1, sizeof(*families));
        if (!families) goto cleanup;
        vkGetPhysicalDeviceQueueFamilyProperties(devices[i], &count, families);
        for (uint32_t j = 0; j < count; ++j) {
            if ((families[j].queueFlags & VK_QUEUE_GRAPHICS_BIT) && families[j].queueCount) {
                physical = devices[i]; family = j; selected = p; break;
            }
        }
        free(families);
    }
    free(devices); devices = NULL;
    if (physical == VK_NULL_HANDLE) { fprintf(stderr, "no CPU Vulkan device with a graphics queue found\n"); goto cleanup; }
    printf("Vulkan CPU device: %s (vendor 0x%04x, device 0x%04x)\n", selected.deviceName, selected.vendorID, selected.deviceID);
    float priority = 1.0f;
    VkDeviceQueueCreateInfo qci = { .sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
        .queueFamilyIndex = family, .queueCount = 1, .pQueuePriorities = &priority };
    VkDeviceCreateInfo dci = { .sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
        .queueCreateInfoCount = 1, .pQueueCreateInfos = &qci };
    CHECK(vkCreateDevice(physical, &dci, NULL, &device));
    VkQueue queue; vkGetDeviceQueue(device, family, 0, &queue);

    VkFormat format = VK_FORMAT_R8G8B8A8_UNORM;
    VkImageCreateInfo imgi = { .sType = VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
        .imageType = VK_IMAGE_TYPE_2D, .format = format,
        .extent = { WIDTH, HEIGHT, 1 }, .mipLevels = 1, .arrayLayers = 1,
        .samples = VK_SAMPLE_COUNT_1_BIT, .tiling = VK_IMAGE_TILING_OPTIMAL,
        .usage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_TRANSFER_SRC_BIT,
        .sharingMode = VK_SHARING_MODE_EXCLUSIVE, .initialLayout = VK_IMAGE_LAYOUT_UNDEFINED };
    CHECK(vkCreateImage(device, &imgi, NULL, &image));
    VkMemoryRequirements imgr; vkGetImageMemoryRequirements(device, image, &imgr);
    uint32_t image_type = find_memory_type(physical, imgr.memoryTypeBits, VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT);
    if (image_type == UINT32_MAX) image_type = find_memory_type(physical, imgr.memoryTypeBits, 0);
    if (image_type == UINT32_MAX) { fprintf(stderr, "no compatible image memory type\n"); goto cleanup; }
    VkMemoryAllocateInfo mai = { .sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
        .allocationSize = imgr.size, .memoryTypeIndex = image_type };
    CHECK(vkAllocateMemory(device, &mai, NULL, &image_memory));
    CHECK(vkBindImageMemory(device, image, image_memory, 0));

    VkImageViewCreateInfo vci = { .sType = VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
        .image = image, .viewType = VK_IMAGE_VIEW_TYPE_2D, .format = format,
        .subresourceRange = { VK_IMAGE_ASPECT_COLOR_BIT, 0, 1, 0, 1 } };
    CHECK(vkCreateImageView(device, &vci, NULL, &view));
    VkAttachmentDescription attachment = { .format = format, .samples = VK_SAMPLE_COUNT_1_BIT,
        .loadOp = VK_ATTACHMENT_LOAD_OP_CLEAR, .storeOp = VK_ATTACHMENT_STORE_OP_STORE,
        .stencilLoadOp = VK_ATTACHMENT_LOAD_OP_DONT_CARE, .stencilStoreOp = VK_ATTACHMENT_STORE_OP_DONT_CARE,
        .initialLayout = VK_IMAGE_LAYOUT_UNDEFINED, .finalLayout = VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL };
    VkAttachmentReference color_ref = { 0, VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL };
    VkSubpassDescription subpass = { .pipelineBindPoint = VK_PIPELINE_BIND_POINT_GRAPHICS,
        .colorAttachmentCount = 1, .pColorAttachments = &color_ref };
    VkSubpassDependency deps[2] = {
        { .srcSubpass = VK_SUBPASS_EXTERNAL, .dstSubpass = 0,
          .srcStageMask = VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT, .dstStageMask = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
          .dstAccessMask = VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT },
        { .srcSubpass = 0, .dstSubpass = VK_SUBPASS_EXTERNAL,
          .srcStageMask = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT, .dstStageMask = VK_PIPELINE_STAGE_TRANSFER_BIT,
          .srcAccessMask = VK_ACCESS_COLOR_ATTACHMENT_WRITE_BIT, .dstAccessMask = VK_ACCESS_TRANSFER_READ_BIT }
    };
    VkRenderPassCreateInfo rpci = { .sType = VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
        .attachmentCount = 1, .pAttachments = &attachment, .subpassCount = 1, .pSubpasses = &subpass,
        .dependencyCount = 2, .pDependencies = deps };
    CHECK(vkCreateRenderPass(device, &rpci, NULL, &render_pass));
    VkFramebufferCreateInfo fbci = { .sType = VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
        .renderPass = render_pass, .attachmentCount = 1, .pAttachments = &view,
        .width = WIDTH, .height = HEIGHT, .layers = 1 };
    CHECK(vkCreateFramebuffer(device, &fbci, NULL, &framebuffer));

    VkShaderModuleCreateInfo smci = { .sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
        .codeSize = vert_size, .pCode = (const uint32_t *)vert_bytes };
    CHECK(vkCreateShaderModule(device, &smci, NULL, &vert));
    smci.codeSize = frag_size; smci.pCode = (const uint32_t *)frag_bytes;
    CHECK(vkCreateShaderModule(device, &smci, NULL, &frag));
    VkPipelineShaderStageCreateInfo stages[2] = {
        { .sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO, .stage = VK_SHADER_STAGE_VERTEX_BIT, .module = vert, .pName = "main" },
        { .sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO, .stage = VK_SHADER_STAGE_FRAGMENT_BIT, .module = frag, .pName = "main" }
    };
    VkPipelineVertexInputStateCreateInfo vertex_input = { .sType = VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO };
    VkPipelineInputAssemblyStateCreateInfo assembly = { .sType = VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
        .topology = VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST };
    VkViewport viewport = { 0, 0, (float)WIDTH, (float)HEIGHT, 0, 1 };
    VkRect2D scissor = { { 0, 0 }, { WIDTH, HEIGHT } };
    VkPipelineViewportStateCreateInfo vp = { .sType = VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
        .viewportCount = 1, .pViewports = &viewport, .scissorCount = 1, .pScissors = &scissor };
    VkPipelineRasterizationStateCreateInfo raster = { .sType = VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
        .polygonMode = VK_POLYGON_MODE_FILL, .cullMode = VK_CULL_MODE_NONE, .frontFace = VK_FRONT_FACE_COUNTER_CLOCKWISE,
        .lineWidth = 1.0f };
    VkPipelineMultisampleStateCreateInfo ms = { .sType = VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
        .rasterizationSamples = VK_SAMPLE_COUNT_1_BIT };
    VkPipelineColorBlendAttachmentState blend_attachment = { .colorWriteMask = VK_COLOR_COMPONENT_R_BIT | VK_COLOR_COMPONENT_G_BIT |
        VK_COLOR_COMPONENT_B_BIT | VK_COLOR_COMPONENT_A_BIT };
    VkPipelineColorBlendStateCreateInfo blend = { .sType = VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
        .attachmentCount = 1, .pAttachments = &blend_attachment };
    VkPipelineLayoutCreateInfo plci = { .sType = VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO };
    CHECK(vkCreatePipelineLayout(device, &plci, NULL, &pipeline_layout));
    VkGraphicsPipelineCreateInfo gpci = { .sType = VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
        .stageCount = 2, .pStages = stages, .pVertexInputState = &vertex_input, .pInputAssemblyState = &assembly,
        .pViewportState = &vp, .pRasterizationState = &raster, .pMultisampleState = &ms,
        .pColorBlendState = &blend, .layout = pipeline_layout, .renderPass = render_pass, .subpass = 0 };
    CHECK(vkCreateGraphicsPipelines(device, VK_NULL_HANDLE, 1, &gpci, NULL, &pipeline));

    VkDeviceSize bytes = (VkDeviceSize)WIDTH * HEIGHT * PIXEL_BYTES;
    VkBufferCreateInfo bci = { .sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
        .size = bytes, .usage = VK_BUFFER_USAGE_TRANSFER_DST_BIT, .sharingMode = VK_SHARING_MODE_EXCLUSIVE };
    CHECK(vkCreateBuffer(device, &bci, NULL, &staging));
    VkMemoryRequirements bmr; vkGetBufferMemoryRequirements(device, staging, &bmr);
    uint32_t staging_type = find_memory_type(physical, bmr.memoryTypeBits,
        VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT);
    if (staging_type == UINT32_MAX) { fprintf(stderr, "no host-visible coherent staging memory type\n"); goto cleanup; }
    mai.allocationSize = bmr.size; mai.memoryTypeIndex = staging_type;
    CHECK(vkAllocateMemory(device, &mai, NULL, &staging_memory));
    CHECK(vkBindBufferMemory(device, staging, staging_memory, 0));

    VkCommandPoolCreateInfo cpci = { .sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO, .queueFamilyIndex = family };
    CHECK(vkCreateCommandPool(device, &cpci, NULL, &command_pool));
    VkCommandBuffer command;
    VkCommandBufferAllocateInfo cbai = { .sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
        .commandPool = command_pool, .level = VK_COMMAND_BUFFER_LEVEL_PRIMARY, .commandBufferCount = 1 };
    CHECK(vkAllocateCommandBuffers(device, &cbai, &command));
    VkCommandBufferBeginInfo begin = { .sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
        .flags = VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT };
    CHECK(vkBeginCommandBuffer(command, &begin));
    VkClearValue clear = { .color = { .float32 = { 0.0f, 0.0f, 0.0f, 1.0f } } };
    VkRenderPassBeginInfo rbi = { .sType = VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
        .renderPass = render_pass, .framebuffer = framebuffer, .renderArea = { { 0, 0 }, { WIDTH, HEIGHT } },
        .clearValueCount = 1, .pClearValues = &clear };
    vkCmdBeginRenderPass(command, &rbi, VK_SUBPASS_CONTENTS_INLINE);
    vkCmdBindPipeline(command, VK_PIPELINE_BIND_POINT_GRAPHICS, pipeline);
    vkCmdDraw(command, 3, 1, 0, 0);
    vkCmdEndRenderPass(command);
    VkBufferImageCopy copy = { .imageSubresource = { VK_IMAGE_ASPECT_COLOR_BIT, 0, 0, 1 },
        .imageExtent = { WIDTH, HEIGHT, 1 } };
    vkCmdCopyImageToBuffer(command, image, VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL, staging, 1, &copy);
    VkBufferMemoryBarrier host_barrier = { .sType = VK_STRUCTURE_TYPE_BUFFER_MEMORY_BARRIER,
        .srcAccessMask = VK_ACCESS_TRANSFER_WRITE_BIT, .dstAccessMask = VK_ACCESS_HOST_READ_BIT,
        .srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED, .dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED,
        .buffer = staging, .offset = 0, .size = bytes };
    vkCmdPipelineBarrier(command, VK_PIPELINE_STAGE_TRANSFER_BIT, VK_PIPELINE_STAGE_HOST_BIT, 0,
        0, NULL, 1, &host_barrier, 0, NULL);
    CHECK(vkEndCommandBuffer(command));
    VkSubmitInfo submit = { .sType = VK_STRUCTURE_TYPE_SUBMIT_INFO, .commandBufferCount = 1, .pCommandBuffers = &command };
    CHECK(vkQueueSubmit(queue, 1, &submit, VK_NULL_HANDLE));
    CHECK(vkQueueWaitIdle(queue)); queue_idle = 1;
    void *mapped = NULL;
    CHECK(vkMapMemory(device, staging_memory, 0, bytes, 0, &mapped));
    const unsigned char *pixels = mapped;
    size_t center = ((size_t)(HEIGHT / 2) * WIDTH + WIDTH / 2) * PIXEL_BYTES;
    size_t corner = 0;
    int pass = pixels[center] == 255 && pixels[center + 1] == 0 && pixels[center + 2] == 0 && pixels[center + 3] == 255 &&
        pixels[corner] == 0 && pixels[corner + 1] == 0 && pixels[corner + 2] == 0 && pixels[corner + 3] == 255;
    if (!pass) fprintf(stderr, "draw readback mismatch: center RGBA=(%u,%u,%u,%u), background=(%u,%u,%u,%u)\n",
        pixels[center], pixels[center+1], pixels[center+2], pixels[center+3],
        pixels[corner], pixels[corner+1], pixels[corner+2], pixels[corner+3]);
    else { printf("offscreen triangle draw verified: center pixel red; corner pixel remains background\n"); status = EXIT_SUCCESS; }
    vkUnmapMemory(device, staging_memory);

cleanup:
    free(devices);
    if (device && !queue_idle && !vk_ok("vkDeviceWaitIdle during cleanup", vkDeviceWaitIdle(device))) status = EXIT_FAILURE;
    if (device) {
        if (command_pool) vkDestroyCommandPool(device, command_pool, NULL);
        if (pipeline) vkDestroyPipeline(device, pipeline, NULL);
        if (pipeline_layout) vkDestroyPipelineLayout(device, pipeline_layout, NULL);
        if (framebuffer) vkDestroyFramebuffer(device, framebuffer, NULL);
        if (render_pass) vkDestroyRenderPass(device, render_pass, NULL);
        if (vert) vkDestroyShaderModule(device, vert, NULL);
        if (frag) vkDestroyShaderModule(device, frag, NULL);
        if (view) vkDestroyImageView(device, view, NULL);
        if (image) vkDestroyImage(device, image, NULL);
        if (staging) vkDestroyBuffer(device, staging, NULL);
        if (image_memory) vkFreeMemory(device, image_memory, NULL);
        if (staging_memory) vkFreeMemory(device, staging_memory, NULL);
        vkDestroyDevice(device, NULL);
    }
    if (instance) vkDestroyInstance(instance, NULL);
    free(vert_bytes); free(frag_bytes);
    return status;
}
