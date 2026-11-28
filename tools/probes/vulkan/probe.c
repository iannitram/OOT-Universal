#include <vulkan/vulkan.h>

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>

#define EXPECTED_VALUE UINT32_C(0x13579bdf)

static int report_vk(const char *operation, VkResult result) {
    if (result == VK_SUCCESS) {
        return 1;
    }
    fprintf(stderr, "%s failed with VkResult %d\n", operation, result);
    return 0;
}

#define VK_CHECK(expression)                                      \
    do {                                                          \
        VkResult check_result = (expression);                     \
        if (!report_vk(#expression, check_result)) {              \
            goto cleanup;                                         \
        }                                                         \
    } while (0)

static unsigned char *read_file(const char *path, size_t *size_out) {
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        perror(path);
        return NULL;
    }
    if (fseek(file, 0, SEEK_END) != 0) {
        perror("fseek");
        fclose(file);
        return NULL;
    }
    long length = ftell(file);
    if (length <= 0 || (length % 4) != 0 || fseek(file, 0, SEEK_SET) != 0) {
        fprintf(stderr, "invalid SPIR-V file length\n");
        fclose(file);
        return NULL;
    }
    unsigned char *bytes = malloc((size_t)length);
    if (bytes == NULL) {
        fprintf(stderr, "could not allocate shader buffer\n");
        fclose(file);
        return NULL;
    }
    size_t read_count = fread(bytes, 1, (size_t)length, file);
    int close_result = fclose(file);
    if (read_count != (size_t)length || close_result != 0) {
        fprintf(stderr, "could not read complete SPIR-V file\n");
        free(bytes);
        return NULL;
    }
    *size_out = (size_t)length;
    return bytes;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s <shader.spv>\n", argv[0]);
        return EXIT_FAILURE;
    }

    int exit_code = EXIT_FAILURE;
    size_t shader_size = 0;
    unsigned char *shader_bytes = read_file(argv[1], &shader_size);
    if (shader_bytes == NULL) {
        return EXIT_FAILURE;
    }

    VkInstance instance = VK_NULL_HANDLE;
    VkDevice device = VK_NULL_HANDLE;
    VkBuffer buffer = VK_NULL_HANDLE;
    VkDeviceMemory buffer_memory = VK_NULL_HANDLE;
    VkShaderModule shader = VK_NULL_HANDLE;
    VkDescriptorSetLayout descriptor_layout = VK_NULL_HANDLE;
    VkPipelineLayout pipeline_layout = VK_NULL_HANDLE;
    VkPipeline pipeline = VK_NULL_HANDLE;
    VkDescriptorPool descriptor_pool = VK_NULL_HANDLE;
    VkCommandPool command_pool = VK_NULL_HANDLE;
    VkPhysicalDevice physical_device = VK_NULL_HANDLE;
    int device_idle = 0;

    VkApplicationInfo app_info = {
        .sType = VK_STRUCTURE_TYPE_APPLICATION_INFO,
        .pApplicationName = "synthetic-vulkan-compute-probe",
        .applicationVersion = 1,
        .pEngineName = "standalone-probe",
        .engineVersion = 1,
        .apiVersion = VK_API_VERSION_1_0,
    };
    VkInstanceCreateInfo instance_info = {
        .sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
        .pApplicationInfo = &app_info,
    };
    VK_CHECK(vkCreateInstance(&instance_info, NULL, &instance));

    uint32_t physical_count = 0;
    VK_CHECK(vkEnumeratePhysicalDevices(instance, &physical_count, NULL));
    if (physical_count == 0) {
        fprintf(stderr, "no Vulkan physical devices found\n");
        goto cleanup;
    }
    VkPhysicalDevice *physical_devices = calloc(physical_count, sizeof(*physical_devices));
    if (physical_devices == NULL) {
        fprintf(stderr, "could not allocate physical-device list\n");
        goto cleanup;
    }
    VkResult enumerate_result = vkEnumeratePhysicalDevices(instance, &physical_count, physical_devices);
    if (!report_vk("vkEnumeratePhysicalDevices", enumerate_result)) {
        free(physical_devices);
        goto cleanup;
    }

    uint32_t queue_family = UINT32_MAX;
    VkPhysicalDeviceProperties selected_properties = {0};
    for (uint32_t i = 0; i < physical_count && physical_device == VK_NULL_HANDLE; ++i) {
        VkPhysicalDeviceProperties properties;
        vkGetPhysicalDeviceProperties(physical_devices[i], &properties);
        if (properties.deviceType != VK_PHYSICAL_DEVICE_TYPE_CPU) {
            continue;
        }
        uint32_t family_count = 0;
        vkGetPhysicalDeviceQueueFamilyProperties(physical_devices[i], &family_count, NULL);
        VkQueueFamilyProperties *families = calloc(family_count, sizeof(*families));
        if (families == NULL) {
            fprintf(stderr, "could not allocate queue-family list\n");
            free(physical_devices);
            goto cleanup;
        }
        vkGetPhysicalDeviceQueueFamilyProperties(physical_devices[i], &family_count, families);
        for (uint32_t j = 0; j < family_count; ++j) {
            if ((families[j].queueFlags & VK_QUEUE_COMPUTE_BIT) != 0 && families[j].queueCount > 0) {
                physical_device = physical_devices[i];
                queue_family = j;
                selected_properties = properties;
                break;
            }
        }
        free(families);
    }
    free(physical_devices);
    if (physical_device == VK_NULL_HANDLE) {
        fprintf(stderr, "no CPU Vulkan device with a compute queue found\n");
        goto cleanup;
    }
    printf("Vulkan CPU device: %s (vendor 0x%04x, device 0x%04x)\n",
           selected_properties.deviceName,
           selected_properties.vendorID,
           selected_properties.deviceID);

    float queue_priority = 1.0f;
    VkDeviceQueueCreateInfo queue_info = {
        .sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
        .queueFamilyIndex = queue_family,
        .queueCount = 1,
        .pQueuePriorities = &queue_priority,
    };
    VkDeviceCreateInfo device_info = {
        .sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
        .queueCreateInfoCount = 1,
        .pQueueCreateInfos = &queue_info,
    };
    VK_CHECK(vkCreateDevice(physical_device, &device_info, NULL, &device));
    VkQueue queue = VK_NULL_HANDLE;
    vkGetDeviceQueue(device, queue_family, 0, &queue);

    VkBufferCreateInfo buffer_info = {
        .sType = VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
        .size = sizeof(uint32_t),
        .usage = VK_BUFFER_USAGE_STORAGE_BUFFER_BIT,
        .sharingMode = VK_SHARING_MODE_EXCLUSIVE,
    };
    VK_CHECK(vkCreateBuffer(device, &buffer_info, NULL, &buffer));
    VkMemoryRequirements memory_requirements;
    vkGetBufferMemoryRequirements(device, buffer, &memory_requirements);
    VkPhysicalDeviceMemoryProperties memory_properties;
    vkGetPhysicalDeviceMemoryProperties(physical_device, &memory_properties);
    uint32_t memory_type = UINT32_MAX;
    for (uint32_t i = 0; i < memory_properties.memoryTypeCount; ++i) {
        VkMemoryPropertyFlags flags = memory_properties.memoryTypes[i].propertyFlags;
        if ((memory_requirements.memoryTypeBits & (1u << i)) != 0 &&
            (flags & (VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT)) ==
                (VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT)) {
            memory_type = i;
            break;
        }
    }
    if (memory_type == UINT32_MAX) {
        fprintf(stderr, "no host-visible coherent memory type for readback buffer\n");
        goto cleanup;
    }
    VkMemoryAllocateInfo memory_info = {
        .sType = VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
        .allocationSize = memory_requirements.size,
        .memoryTypeIndex = memory_type,
    };
    VK_CHECK(vkAllocateMemory(device, &memory_info, NULL, &buffer_memory));
    VK_CHECK(vkBindBufferMemory(device, buffer, buffer_memory, 0));
    void *mapped = NULL;
    VK_CHECK(vkMapMemory(device, buffer_memory, 0, sizeof(uint32_t), 0, &mapped));
    *(uint32_t *)mapped = 0;
    vkUnmapMemory(device, buffer_memory);

    VkShaderModuleCreateInfo shader_info = {
        .sType = VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
        .codeSize = shader_size,
        .pCode = (const uint32_t *)shader_bytes,
    };
    VK_CHECK(vkCreateShaderModule(device, &shader_info, NULL, &shader));

    VkDescriptorSetLayoutBinding descriptor_binding = {
        .binding = 0,
        .descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
        .descriptorCount = 1,
        .stageFlags = VK_SHADER_STAGE_COMPUTE_BIT,
    };
    VkDescriptorSetLayoutCreateInfo descriptor_layout_info = {
        .sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
        .bindingCount = 1,
        .pBindings = &descriptor_binding,
    };
    VK_CHECK(vkCreateDescriptorSetLayout(device, &descriptor_layout_info, NULL, &descriptor_layout));
    VkPipelineLayoutCreateInfo pipeline_layout_info = {
        .sType = VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
        .setLayoutCount = 1,
        .pSetLayouts = &descriptor_layout,
    };
    VK_CHECK(vkCreatePipelineLayout(device, &pipeline_layout_info, NULL, &pipeline_layout));
    VkPipelineShaderStageCreateInfo stage_info = {
        .sType = VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
        .stage = VK_SHADER_STAGE_COMPUTE_BIT,
        .module = shader,
        .pName = "main",
    };
    VkComputePipelineCreateInfo pipeline_info = {
        .sType = VK_STRUCTURE_TYPE_COMPUTE_PIPELINE_CREATE_INFO,
        .stage = stage_info,
        .layout = pipeline_layout,
    };
    VK_CHECK(vkCreateComputePipelines(device, VK_NULL_HANDLE, 1, &pipeline_info, NULL, &pipeline));

    VkDescriptorPoolSize pool_size = {
        .type = VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
        .descriptorCount = 1,
    };
    VkDescriptorPoolCreateInfo pool_info = {
        .sType = VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO,
        .maxSets = 1,
        .poolSizeCount = 1,
        .pPoolSizes = &pool_size,
    };
    VK_CHECK(vkCreateDescriptorPool(device, &pool_info, NULL, &descriptor_pool));
    VkDescriptorSetAllocateInfo set_info = {
        .sType = VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO,
        .descriptorPool = descriptor_pool,
        .descriptorSetCount = 1,
        .pSetLayouts = &descriptor_layout,
    };
    VkDescriptorSet descriptor_set = VK_NULL_HANDLE;
    VK_CHECK(vkAllocateDescriptorSets(device, &set_info, &descriptor_set));
    VkDescriptorBufferInfo descriptor_buffer_info = {
        .buffer = buffer,
        .offset = 0,
        .range = sizeof(uint32_t),
    };
    VkWriteDescriptorSet descriptor_write = {
        .sType = VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
        .dstSet = descriptor_set,
        .dstBinding = 0,
        .descriptorCount = 1,
        .descriptorType = VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
        .pBufferInfo = &descriptor_buffer_info,
    };
    vkUpdateDescriptorSets(device, 1, &descriptor_write, 0, NULL);

    VkCommandPoolCreateInfo command_pool_info = {
        .sType = VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
        .queueFamilyIndex = queue_family,
    };
    VK_CHECK(vkCreateCommandPool(device, &command_pool_info, NULL, &command_pool));
    VkCommandBufferAllocateInfo command_info = {
        .sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
        .commandPool = command_pool,
        .level = VK_COMMAND_BUFFER_LEVEL_PRIMARY,
        .commandBufferCount = 1,
    };
    VkCommandBuffer command_buffer = VK_NULL_HANDLE;
    VK_CHECK(vkAllocateCommandBuffers(device, &command_info, &command_buffer));
    VkCommandBufferBeginInfo begin_info = {
        .sType = VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
        .flags = VK_COMMAND_BUFFER_USAGE_ONE_TIME_SUBMIT_BIT,
    };
    VK_CHECK(vkBeginCommandBuffer(command_buffer, &begin_info));
    vkCmdBindPipeline(command_buffer, VK_PIPELINE_BIND_POINT_COMPUTE, pipeline);
    vkCmdBindDescriptorSets(command_buffer, VK_PIPELINE_BIND_POINT_COMPUTE, pipeline_layout,
                            0, 1, &descriptor_set, 0, NULL);
    vkCmdDispatch(command_buffer, 1, 1, 1);
    VkBufferMemoryBarrier readback_barrier = {
        .sType = VK_STRUCTURE_TYPE_BUFFER_MEMORY_BARRIER,
        .srcAccessMask = VK_ACCESS_SHADER_WRITE_BIT,
        .dstAccessMask = VK_ACCESS_HOST_READ_BIT,
        .srcQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED,
        .dstQueueFamilyIndex = VK_QUEUE_FAMILY_IGNORED,
        .buffer = buffer,
        .offset = 0,
        .size = sizeof(uint32_t),
    };
    vkCmdPipelineBarrier(command_buffer, VK_PIPELINE_STAGE_COMPUTE_SHADER_BIT,
                         VK_PIPELINE_STAGE_HOST_BIT, 0, 0, NULL, 1,
                         &readback_barrier, 0, NULL);
    VK_CHECK(vkEndCommandBuffer(command_buffer));
    VkSubmitInfo submit_info = {
        .sType = VK_STRUCTURE_TYPE_SUBMIT_INFO,
        .commandBufferCount = 1,
        .pCommandBuffers = &command_buffer,
    };
    VK_CHECK(vkQueueSubmit(queue, 1, &submit_info, VK_NULL_HANDLE));
    VK_CHECK(vkQueueWaitIdle(queue));
    device_idle = 1;
    VK_CHECK(vkMapMemory(device, buffer_memory, 0, sizeof(uint32_t), 0, &mapped));
    uint32_t actual_value = *(const uint32_t *)mapped;
    vkUnmapMemory(device, buffer_memory);
    if (actual_value != EXPECTED_VALUE) {
        fprintf(stderr, "readback mismatch: expected 0x%08" PRIx32 ", got 0x%08" PRIx32 "\n",
                EXPECTED_VALUE, actual_value);
        goto cleanup;
    }
    printf("compute dispatch wrote and mapped readback verified: 0x%08" PRIx32 "\n", actual_value);
    exit_code = EXIT_SUCCESS;

cleanup:
    if (device != VK_NULL_HANDLE && !device_idle) {
        VkResult wait_result = vkDeviceWaitIdle(device);
        if (!report_vk("vkDeviceWaitIdle during cleanup", wait_result)) {
            exit_code = EXIT_FAILURE;
        }
    }
    if (device != VK_NULL_HANDLE) {
        if (command_pool != VK_NULL_HANDLE) vkDestroyCommandPool(device, command_pool, NULL);
        if (descriptor_pool != VK_NULL_HANDLE) vkDestroyDescriptorPool(device, descriptor_pool, NULL);
        if (pipeline != VK_NULL_HANDLE) vkDestroyPipeline(device, pipeline, NULL);
        if (pipeline_layout != VK_NULL_HANDLE) vkDestroyPipelineLayout(device, pipeline_layout, NULL);
        if (descriptor_layout != VK_NULL_HANDLE) vkDestroyDescriptorSetLayout(device, descriptor_layout, NULL);
        if (shader != VK_NULL_HANDLE) vkDestroyShaderModule(device, shader, NULL);
        if (buffer != VK_NULL_HANDLE) vkDestroyBuffer(device, buffer, NULL);
        if (buffer_memory != VK_NULL_HANDLE) vkFreeMemory(device, buffer_memory, NULL);
        vkDestroyDevice(device, NULL);
    }
    if (instance != VK_NULL_HANDLE) vkDestroyInstance(instance, NULL);
    free(shader_bytes);
    return exit_code;
}
