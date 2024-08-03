use ash::prelude::VkResult;
use ash::vk;
use std::mem;

use crate::definitions::*;
use crate::ffi;

/// Constructor a new `Allocator` using the provided options.
pub fn new_vma_allocator(mut create_info: AllocatorCreateInfo) -> VkResult<ffi::VmaAllocator> {
    unsafe extern "system" fn get_instance_proc_addr_stub(
        _instance: ash::vk::Instance,
        _p_name: *const ::std::os::raw::c_char,
    ) -> ash::vk::PFN_vkVoidFunction {
        panic!("VMA_DYNAMIC_VULKAN_FUNCTIONS is unsupported")
    }

    unsafe extern "system" fn get_get_device_proc_stub(
        _device: ash::vk::Device,
        _p_name: *const ::std::os::raw::c_char,
    ) -> ash::vk::PFN_vkVoidFunction {
        panic!("VMA_DYNAMIC_VULKAN_FUNCTIONS is unsupported")
    }

    #[cfg(feature = "loaded")]
    let routed_functions = ffi::VmaVulkanFunctions {
        vkGetInstanceProcAddr: get_instance_proc_addr_stub,
        vkGetDeviceProcAddr: get_get_device_proc_stub,
        vkGetPhysicalDeviceProperties: create_info
            .instance
            .fp_v1_0()
            .get_physical_device_properties,
        vkGetPhysicalDeviceMemoryProperties: create_info
            .instance
            .fp_v1_0()
            .get_physical_device_memory_properties,
        vkAllocateMemory: create_info.device.fp_v1_0().allocate_memory,
        vkFreeMemory: create_info.device.fp_v1_0().free_memory,
        vkMapMemory: create_info.device.fp_v1_0().map_memory,
        vkUnmapMemory: create_info.device.fp_v1_0().unmap_memory,
        vkFlushMappedMemoryRanges: create_info.device.fp_v1_0().flush_mapped_memory_ranges,
        vkInvalidateMappedMemoryRanges: create_info
            .device
            .fp_v1_0()
            .invalidate_mapped_memory_ranges,
        vkBindBufferMemory: create_info.device.fp_v1_0().bind_buffer_memory,
        vkBindImageMemory: create_info.device.fp_v1_0().bind_image_memory,
        vkGetBufferMemoryRequirements: create_info.device.fp_v1_0().get_buffer_memory_requirements,
        vkGetImageMemoryRequirements: create_info.device.fp_v1_0().get_image_memory_requirements,
        vkCreateBuffer: create_info.device.fp_v1_0().create_buffer,
        vkDestroyBuffer: create_info.device.fp_v1_0().destroy_buffer,
        vkCreateImage: create_info.device.fp_v1_0().create_image,
        vkDestroyImage: create_info.device.fp_v1_0().destroy_image,
        vkCmdCopyBuffer: create_info.device.fp_v1_0().cmd_copy_buffer,
        vkGetBufferMemoryRequirements2KHR: create_info
            .device
            .fp_v1_1()
            .get_buffer_memory_requirements2,
        vkGetImageMemoryRequirements2KHR: create_info
            .device
            .fp_v1_1()
            .get_image_memory_requirements2,
        vkBindBufferMemory2KHR: create_info.device.fp_v1_1().bind_buffer_memory2,
        vkBindImageMemory2KHR: create_info.device.fp_v1_1().bind_image_memory2,
        vkGetPhysicalDeviceMemoryProperties2KHR: create_info
            .instance
            .fp_v1_1()
            .get_physical_device_memory_properties2,
        vkGetDeviceBufferMemoryRequirements: create_info
            .device
            .fp_v1_3()
            .get_device_buffer_memory_requirements,
        vkGetDeviceImageMemoryRequirements: create_info
            .device
            .fp_v1_3()
            .get_device_image_memory_requirements,
    };
    #[cfg(feature = "loaded")]
    {
        create_info.inner.pVulkanFunctions = &routed_functions;
    }
    unsafe {
        let mut handle: ffi::VmaAllocator = mem::zeroed();
        ffi::vmaCreateAllocator(&create_info.inner as *const _, &mut handle).result()?;

        Ok(handle)
    }
}
