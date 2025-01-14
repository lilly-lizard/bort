//! See [here](https://asawicki.info/news_1740_vulkan_memory_types_on_pc_and_how_to_use_them) for advice
//! on vulkan memory types on PC.

use crate::{device::Device, AllocationInfo, AllocatorAccess, ApiVersion, DefragmentationContext};
use ash::{
    khr::{bind_memory2, get_memory_requirements2, get_physical_device_properties2, maintenance4},
    prelude::VkResult,
    vk::{
        self, PFN_vkBindBufferMemory2, PFN_vkBindImageMemory2, PFN_vkGetBufferMemoryRequirements2,
        PFN_vkGetDeviceBufferMemoryRequirements, PFN_vkGetDeviceImageMemoryRequirements,
        PFN_vkGetImageMemoryRequirements2, PFN_vkGetPhysicalDeviceMemoryProperties2,
        KHR_BIND_MEMORY2_NAME, KHR_GET_MEMORY_REQUIREMENTS2_NAME,
        KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME, KHR_MAINTENANCE4_NAME,
    },
};
use bort_vma::{ffi, AllocatorCreateInfo};
use log::warn;
use std::{mem, sync::Arc};

/// so it's easy to find all allocation callback args, just in case I want to use them in the future.
pub const ALLOCATION_CALLBACK_NONE: Option<&ash::vk::AllocationCallbacks> = None;

// ~~ Memory Allocator ~~

pub struct MemoryAllocator {
    /// pointer to internal VmaAllocator instance
    handle: ffi::VmaAllocator,

    // dependencies
    device: Arc<Device>,
}

/// Constructor a new `Allocator` using the provided options.
pub(crate) fn new_vma_allocator(
    device: &Device,
    mut create_info: AllocatorCreateInfo,
) -> VkResult<ffi::VmaAllocator> {
    if device.instance().max_api_version() < ApiVersion::V1_1
        && (!device
            .enabled_extensions()
            .contains(&KHR_GET_MEMORY_REQUIREMENTS2_NAME.to_owned())
            || !device
                .enabled_extensions()
                .contains(&KHR_BIND_MEMORY2_NAME.to_owned())
            || !device
                .instance()
                .enabled_extensions()
                .contains(&KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME.to_owned()))
    {
        warn!("vma requires the following extensions to be enabled when using vulkan 1.0:");
        warn!("\tKHR_GET_MEMORY_REQUIREMENTS2");
        warn!("\tKHR_BIND_MEMORY2");
        warn!("\tKHR_GET_PHYSICAL_DEVICE_PROPERTIES2");
    }

    if device.instance().max_api_version() < ApiVersion::V1_3
        && !device
            .enabled_extensions()
            .contains(&KHR_MAINTENANCE4_NAME.to_owned())
    {
        warn!("vma requires the following extensions to be enabled when using vulkan < 3.0:");
        warn!("\tKHR_GET_MEMORY_REQUIREMENTS2");
    }

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
    let routed_functions = {
        let (get_buffer_memory_requirements2, get_image_memory_requirements2) =
            get_fns_get_memory_requirements2(device, &create_info);
        let (bind_buffer_memory2, bind_image_memory2) = get_fns_bind_memory2(device, &create_info);
        let get_physical_device_memory_properties2 =
            get_fns_get_physical_device_properties2(device, &create_info);
        let (get_device_buffer_memory_requirements, get_device_image_memory_requirements) =
            get_fns_maintenance4(device, &create_info);

        ffi::VmaVulkanFunctions {
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
            vkGetBufferMemoryRequirements: create_info
                .device
                .fp_v1_0()
                .get_buffer_memory_requirements,
            vkGetImageMemoryRequirements: create_info
                .device
                .fp_v1_0()
                .get_image_memory_requirements,
            vkCreateBuffer: create_info.device.fp_v1_0().create_buffer,
            vkDestroyBuffer: create_info.device.fp_v1_0().destroy_buffer,
            vkCreateImage: create_info.device.fp_v1_0().create_image,
            vkDestroyImage: create_info.device.fp_v1_0().destroy_image,
            vkCmdCopyBuffer: create_info.device.fp_v1_0().cmd_copy_buffer,
            vkGetBufferMemoryRequirements2KHR: get_buffer_memory_requirements2,
            vkGetImageMemoryRequirements2KHR: get_image_memory_requirements2,
            vkBindBufferMemory2KHR: bind_buffer_memory2,
            vkBindImageMemory2KHR: bind_image_memory2,
            vkGetPhysicalDeviceMemoryProperties2KHR: get_physical_device_memory_properties2,
            vkGetDeviceBufferMemoryRequirements: get_device_buffer_memory_requirements,
            vkGetDeviceImageMemoryRequirements: get_device_image_memory_requirements,
        }
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

fn get_fns_get_memory_requirements2(
    device: &Device,
    create_info: &AllocatorCreateInfo<'_>,
) -> (
    PFN_vkGetBufferMemoryRequirements2,
    PFN_vkGetImageMemoryRequirements2,
) {
    if device.instance().max_api_version() < ApiVersion::V1_1
        && device
            .enabled_extensions()
            .contains(&KHR_GET_MEMORY_REQUIREMENTS2_NAME.to_owned())
    {
        let extension_device =
            get_memory_requirements2::Device::new(device.instance().inner(), device.inner());
        (
            extension_device.fp().get_buffer_memory_requirements2_khr,
            extension_device.fp().get_image_memory_requirements2_khr,
        )
    } else {
        (
            create_info.device.fp_v1_1().get_buffer_memory_requirements2,
            create_info.device.fp_v1_1().get_image_memory_requirements2,
        )
    }
}

fn get_fns_bind_memory2(
    device: &Device,
    create_info: &AllocatorCreateInfo<'_>,
) -> (PFN_vkBindBufferMemory2, PFN_vkBindImageMemory2) {
    if device.instance().max_api_version() < ApiVersion::V1_1
        && device
            .enabled_extensions()
            .contains(&KHR_BIND_MEMORY2_NAME.to_owned())
    {
        let extension_device = bind_memory2::Device::new(device.instance().inner(), device.inner());
        (
            extension_device.fp().bind_buffer_memory2_khr,
            extension_device.fp().bind_image_memory2_khr,
        )
    } else {
        (
            create_info.device.fp_v1_1().bind_buffer_memory2,
            create_info.device.fp_v1_1().bind_image_memory2,
        )
    }
}

fn get_fns_get_physical_device_properties2(
    device: &Device,
    create_info: &AllocatorCreateInfo<'_>,
) -> PFN_vkGetPhysicalDeviceMemoryProperties2 {
    if device.instance().max_api_version() < ApiVersion::V1_1
        && device
            .enabled_extensions()
            .contains(&KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_NAME.to_owned())
    {
        let extension_device = get_physical_device_properties2::Instance::new(
            device.instance().entry(),
            device.instance().inner(),
        );
        extension_device
            .fp()
            .get_physical_device_memory_properties2_khr
    } else {
        create_info
            .instance
            .fp_v1_1()
            .get_physical_device_memory_properties2
    }
}

fn get_fns_maintenance4(
    device: &Device,
    create_info: &AllocatorCreateInfo<'_>,
) -> (
    PFN_vkGetDeviceBufferMemoryRequirements,
    PFN_vkGetDeviceImageMemoryRequirements,
) {
    if device.instance().max_api_version() < ApiVersion::V1_3
        && device
            .enabled_extensions()
            .contains(&KHR_MAINTENANCE4_NAME.to_owned())
    {
        let extension_device = maintenance4::Device::new(device.instance().inner(), device.inner());
        (
            extension_device
                .fp()
                .get_device_buffer_memory_requirements_khr,
            extension_device
                .fp()
                .get_device_image_memory_requirements_khr,
        )
    } else {
        (
            create_info
                .device
                .fp_v1_3()
                .get_device_buffer_memory_requirements,
            create_info
                .device
                .fp_v1_3()
                .get_device_image_memory_requirements,
        )
    }
}

impl MemoryAllocator {
    pub fn new(device: Arc<Device>) -> VkResult<Self> {
        let api_version_uint = device.instance().max_api_version().as_vk_uint();
        let allocator_info = AllocatorCreateInfo::new(
            device.instance().inner(),
            device.inner(),
            device.physical_device().handle(),
        )
        .vulkan_api_version(api_version_uint);

        unsafe { Self::new_from_create_info(device.clone(), allocator_info) }
    }

    /// # Safety
    /// Make sure your `p_next` chain contains valid pointers.
    pub unsafe fn new_from_create_info(
        device: Arc<Device>,
        create_info: AllocatorCreateInfo,
    ) -> VkResult<Self> {
        let handle = new_vma_allocator(&device, create_info)?;
        Ok(Self { handle, device })
    }

    /// The allocator fetches `ash::vk::PhysicalDeviceProperties` from the physical device.
    /// You can get it here, without fetching it again on your own.
    pub unsafe fn get_physical_device_properties(&self) -> VkResult<vk::PhysicalDeviceProperties> {
        let mut properties = vk::PhysicalDeviceProperties::default();
        ffi::vmaGetPhysicalDeviceProperties(
            self.handle,
            &mut properties as *mut _ as *mut *const _,
        );

        Ok(properties)
    }

    /// The allocator fetches `ash::vk::PhysicalDeviceMemoryProperties` from the physical device.
    /// You can get it here, without fetching it again on your own.
    pub unsafe fn get_memory_properties(&self) -> &vk::PhysicalDeviceMemoryProperties {
        let mut properties: *const vk::PhysicalDeviceMemoryProperties = std::ptr::null();
        ffi::vmaGetMemoryProperties(self.handle, &mut properties);

        &*properties
    }

    /// Sets index of the current frame.
    ///
    /// This function must be used if you make allocations with `AllocationCreateFlags::CAN_BECOME_LOST` and
    /// `AllocationCreateFlags::CAN_MAKE_OTHER_LOST` flags to inform the allocator when a new frame begins.
    /// Allocations queried using `Allocator::get_allocation_info` cannot become lost
    /// in the current frame.
    pub unsafe fn set_current_frame_index(&self, frame_index: u32) {
        ffi::vmaSetCurrentFrameIndex(self.handle, frame_index);
    }

    /// Retrieves statistics from current state of the `Allocator`.
    pub fn calculate_statistics(&self) -> VkResult<ffi::VmaTotalStatistics> {
        unsafe {
            let mut vma_stats: ffi::VmaTotalStatistics = mem::zeroed();
            ffi::vmaCalculateStatistics(self.handle, &mut vma_stats);
            Ok(vma_stats)
        }
    }

    /// Retrieves information about current memory usage and budget for all memory heaps.
    ///
    /// This function is called "get" not "calculate" because it is very fast, suitable to be called
    /// every frame or every allocation. For more detailed statistics use vmaCalculateStatistics().
    ///
    /// Note that when using allocator from multiple threads, returned information may immediately
    /// become outdated.
    pub fn get_heap_budgets(&self) -> VkResult<Vec<ffi::VmaBudget>> {
        unsafe {
            let len = self.get_memory_properties().memory_heap_count as usize;
            let mut vma_budgets: Vec<ffi::VmaBudget> = Vec::with_capacity(len);
            ffi::vmaGetHeapBudgets(self.handle, vma_budgets.as_mut_ptr());
            vma_budgets.set_len(len);
            Ok(vma_budgets)
        }
    }

    /// Frees memory previously allocated using `Allocator::allocate_memory`,
    /// `Allocator::allocate_memory_for_buffer`, or `Allocator::allocate_memory_for_image`.
    pub unsafe fn vma_free_memory(&self, allocation_handle: ffi::VmaAllocation) {
        ffi::vmaFreeMemory(self.handle, allocation_handle);
    }

    /// Frees memory and destroys multiple allocations.
    ///
    /// Word "pages" is just a suggestion to use this function to free pieces of memory used for sparse binding.
    /// It is just a general purpose function to free memory and destroy allocations made using e.g. `Allocator::allocate_memory',
    /// 'Allocator::allocate_memory_pages` and other functions.
    ///
    /// It may be internally optimized to be more efficient than calling 'Allocator::free_memory` `allocations.len()` times.
    ///
    /// Allocations in 'allocations' slice can come from any memory pools and types.
    pub unsafe fn vma_free_memory_pages(&self, allocation_handles: &mut [ffi::VmaAllocation]) {
        ffi::vmaFreeMemoryPages(
            self.handle,
            allocation_handles.len(),
            allocation_handles.as_mut_ptr(),
        );
    }

    /// Returns current information about specified allocation and atomically marks it as used in current frame.
    ///
    /// Current parameters of given allocation are returned in the result object, available through accessors.
    ///
    /// This function also atomically "touches" allocation - marks it as used in current frame,
    /// just like `Allocator::touch_allocation`.
    ///
    /// If the allocation is in lost state, `allocation.get_device_memory` returns `ash::vk::DeviceMemory::null()`.
    ///
    /// Although this function uses atomics and doesn't lock any mutex, so it should be quite efficient,
    /// you can avoid calling it too often.
    ///
    /// If you just want to check if allocation is not lost, `Allocator::touch_allocation` will work faster.
    pub fn vma_get_allocation_info(&self, allocation_handle: ffi::VmaAllocation) -> AllocationInfo {
        unsafe {
            let mut allocation_info: ffi::VmaAllocationInfo = mem::zeroed();
            ffi::vmaGetAllocationInfo(self.handle, allocation_handle, &mut allocation_info);
            allocation_info.into()
        }
    }

    /// Sets user data in given allocation to new value.
    ///
    /// If the allocation was created with `AllocationCreateFlags::USER_DATA_COPY_STRING`,
    /// `user_data` must be either null, or pointer to a null-terminated string. The function
    /// makes local copy of the string and sets it as allocation's user data. String
    /// passed as user data doesn't need to be valid for whole lifetime of the allocation -
    /// you can free it after this call. String previously pointed by allocation's
    /// user data is freed from memory.
    ///
    /// If the flag was not used, the value of pointer `user_data` is just copied to
    /// allocation's user data. It is opaque, so you can use it however you want - e.g.
    /// as a pointer, ordinal number or some handle to you own data.
    pub unsafe fn vma_set_allocation_user_data(
        &self,
        allocation_handle: ffi::VmaAllocation,
        user_data: *mut ::std::os::raw::c_void,
    ) {
        ffi::vmaSetAllocationUserData(self.handle, allocation_handle, user_data);
    }

    /// Maps memory represented by given allocation and returns pointer to it.
    ///
    /// Maps memory represented by given allocation to make it accessible to CPU code.
    /// When succeeded, result is a pointer to first byte of this memory.
    ///
    /// If the allocation is part of bigger `ash::vk::DeviceMemory` block, the pointer is
    /// correctly offseted to the beginning of region assigned to this particular
    /// allocation.
    ///
    /// Mapping is internally reference-counted and synchronized, so despite raw Vulkan
    /// function `ash::vk::Device::MapMemory` cannot be used to map same block of
    /// `ash::vk::DeviceMemory` multiple times simultaneously, it is safe to call this
    /// function on allocations assigned to the same memory block. Actual Vulkan memory
    /// will be mapped on first mapping and unmapped on last unmapping.
    ///
    /// If the function succeeded, you must call `Allocator::unmap_memory` to unmap the
    /// allocation when mapping is no longer needed or before freeing the allocation, at
    /// the latest.
    ///
    /// It also safe to call this function multiple times on the same allocation. You
    /// must call `Allocator::unmap_memory` same number of times as you called
    /// `Allocator::map_memory`.
    ///
    /// It is also safe to call this function on allocation created with
    /// `AllocationCreateFlags::MAPPED` flag. Its memory stays mapped all the time.
    /// You must still call `Allocator::unmap_memory` same number of times as you called
    /// `Allocator::map_memory`. You must not call `Allocator::unmap_memory` additional
    /// time to free the "0-th" mapping made automatically due to `AllocationCreateFlags::MAPPED` flag.
    ///
    /// This function fails when used on allocation made in memory type that is not
    /// `ash::vk::MemoryPropertyFlags::HOST_VISIBLE`.
    ///
    /// This function always fails when called for allocation that was created with
    /// `AllocationCreateFlags::CAN_BECOME_LOST` flag. Such allocations cannot be mapped.
    pub unsafe fn vma_map_memory(
        &self,
        allocation_handle: ffi::VmaAllocation,
    ) -> VkResult<*mut u8> {
        let mut mapped_data: *mut ::std::os::raw::c_void = ::std::ptr::null_mut();
        ffi::vmaMapMemory(self.handle, allocation_handle, &mut mapped_data).result()?;

        Ok(mapped_data as *mut u8)
    }

    /// Unmaps memory represented by given allocation, mapped previously using `Allocator::map_memory`.
    pub unsafe fn vma_unmap_memory(&self, allocation_handle: ffi::VmaAllocation) {
        ffi::vmaUnmapMemory(self.handle, allocation_handle);
    }

    /// Flushes memory of given allocation.
    ///
    /// Calls `ash::vk::Device::FlushMappedMemoryRanges` for memory associated with given range of given allocation.
    ///
    /// - `offset` must be relative to the beginning of allocation.
    /// - `size` can be `ash::vk::WHOLE_SIZE`. It means all memory from `offset` the the end of given allocation.
    /// - `offset` and `size` don't have to be aligned; hey are internally rounded down/up to multiple of `nonCoherentAtomSize`.
    /// - If `size` is 0, this call is ignored.
    /// - If memory type that the `allocation` belongs to is not `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` or it is `ash::vk::MemoryPropertyFlags::HOST_COHERENT`, this call is ignored.
    pub fn vma_flush_allocation(
        &self,
        allocation_handle: ffi::VmaAllocation,
        offset: usize,
        size: usize,
    ) -> VkResult<()> {
        unsafe {
            ffi::vmaFlushAllocation(
                self.handle,
                allocation_handle,
                offset as vk::DeviceSize,
                size as vk::DeviceSize,
            )
            .result()
        }
    }

    /// Invalidates memory of given allocation.
    ///
    /// Calls `ash::vk::Device::invalidate_mapped_memory_ranges` for memory associated with given range of given allocation.
    ///
    /// - `offset` must be relative to the beginning of allocation.
    /// - `size` can be `ash::vk::WHOLE_SIZE`. It means all memory from `offset` the the end of given allocation.
    /// - `offset` and `size` don't have to be aligned. They are internally rounded down/up to multiple of `nonCoherentAtomSize`.
    /// - If `size` is 0, this call is ignored.
    /// - If memory type that the `allocation` belongs to is not `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` or it is `ash::vk::MemoryPropertyFlags::HOST_COHERENT`, this call is ignored.
    pub fn vma_invalidate_allocation(
        &self,
        allocation_handle: ffi::VmaAllocation,
        offset: usize,
        size: usize,
    ) -> VkResult<()> {
        unsafe {
            ffi::vmaInvalidateAllocation(
                self.handle,
                allocation_handle,
                offset as vk::DeviceSize,
                size as vk::DeviceSize,
            )
            .result()
        }
    }

    /// Checks magic number in margins around all allocations in given memory types (in both default and custom pools) in search for corruptions.
    ///
    /// `memory_type_bits` bit mask, where each bit set means that a memory type with that index should be checked.
    ///
    /// Corruption detection is enabled only when `VMA_DEBUG_DETECT_CORRUPTION` macro is defined to nonzero,
    /// `VMA_DEBUG_MARGIN` is defined to nonzero and only for memory types that are `HOST_VISIBLE` and `HOST_COHERENT`.
    ///
    /// Possible error values:
    ///
    /// - `ash::vk::Result::ERROR_FEATURE_NOT_PRESENT` - corruption detection is not enabled for any of specified memory types.
    /// - `ash::vk::Result::ERROR_VALIDATION_FAILED_EXT` - corruption detection has been performed and found memory corruptions around one of the allocations.
    ///   `VMA_ASSERT` is also fired in that case.
    /// - Other value: Error returned by Vulkan, e.g. memory mapping failure.
    pub unsafe fn check_corruption(
        &self,
        memory_types: ash::vk::MemoryPropertyFlags,
    ) -> VkResult<()> {
        ffi::vmaCheckCorruption(self.handle, memory_types.as_raw()).result()
    }

    /// Binds buffer to allocation.
    ///
    /// Binds specified buffer to region of memory represented by specified allocation.
    /// Gets `ash::vk::DeviceMemory` handle and offset from the allocation.
    ///
    /// If you want to create a buffer, allocate memory for it and bind them together separately,
    /// you should use this function for binding instead of `ash::vk::Device::bind_buffer_memory`,
    /// because it ensures proper synchronization so that when a `ash::vk::DeviceMemory` object is
    /// used by multiple allocations, calls to `ash::vk::Device::bind_buffer_memory()` or
    /// `ash::vk::Device::map_memory()` won't happen from multiple threads simultaneously
    /// (which is illegal in Vulkan).
    ///
    /// It is recommended to use function `Allocator::create_buffer` instead of this one.
    pub unsafe fn vma_bind_buffer_memory(
        &self,
        allocation_handle: ffi::VmaAllocation,
        buffer: ash::vk::Buffer,
    ) -> VkResult<()> {
        ffi::vmaBindBufferMemory(self.handle, allocation_handle, buffer).result()
    }

    /// Binds buffer to allocation with additional parameters.
    ///
    /// * `allocation`
    /// * `allocation_local_offset` - Additional offset to be added while binding, relative to the beginning of the `allocation`. Normally it should be 0.
    /// * `buffer`
    /// * `next` - A chain of structures to be attached to `VkBindImageMemoryInfoKHR` structure used internally. Normally it should be null.
    ///
    /// This function is similar to vmaBindImageMemory(), but it provides additional parameters.
    ///
    /// If `pNext` is not null, #VmaAllocator object must have been created with #VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT flag
    /// or with VmaAllocatorCreateInfo::vulkanApiVersion `>= VK_API_VERSION_1_1`. Otherwise the call fails.
    pub unsafe fn vma_bind_buffer_memory2(
        &self,
        allocation_handle: ffi::VmaAllocation,
        allocation_local_offset: vk::DeviceSize,
        buffer: ash::vk::Buffer,
        next: *const ::std::os::raw::c_void,
    ) -> VkResult<()> {
        ffi::vmaBindBufferMemory2(
            self.handle,
            allocation_handle,
            allocation_local_offset,
            buffer,
            next,
        )
        .result()
    }

    /// Binds image to allocation.
    ///
    /// Binds specified image to region of memory represented by specified allocation.
    /// Gets `ash::vk::DeviceMemory` handle and offset from the allocation.
    ///
    /// If you want to create a image, allocate memory for it and bind them together separately,
    /// you should use this function for binding instead of `ash::vk::Device::bind_image_memory`,
    /// because it ensures proper synchronization so that when a `ash::vk::DeviceMemory` object is
    /// used by multiple allocations, calls to `ash::vk::Device::bind_image_memory()` or
    /// `ash::vk::Device::map_memory()` won't happen from multiple threads simultaneously
    /// (which is illegal in Vulkan).
    ///
    /// It is recommended to use function `Allocator::create_image` instead of this one.
    pub unsafe fn vma_bind_image_memory(
        &self,
        allocation_handle: ffi::VmaAllocation,
        image: ash::vk::Image,
    ) -> VkResult<()> {
        ffi::vmaBindImageMemory(self.handle, allocation_handle, image).result()
    }

    /// Binds image to allocation with additional parameters.
    ///
    /// * `allocation`
    /// * `allocation_local_offset` - Additional offset to be added while binding, relative to the beginning of the `allocation`. Normally it should be 0.
    /// * `image`
    /// * `next` - A chain of structures to be attached to `VkBindImageMemoryInfoKHR` structure used internally. Normally it should be null.
    ///
    /// This function is similar to vmaBindImageMemory(), but it provides additional parameters.
    ///
    /// If `pNext` is not null, #VmaAllocator object must have been created with #VMA_ALLOCATOR_CREATE_KHR_BIND_MEMORY2_BIT flag
    /// or with VmaAllocatorCreateInfo::vulkanApiVersion `>= VK_API_VERSION_1_1`. Otherwise the call fails.
    pub unsafe fn vma_bind_image_memory2(
        &self,
        allocation_handle: ffi::VmaAllocation,
        allocation_local_offset: vk::DeviceSize,
        image: ash::vk::Image,
        next: *const ::std::os::raw::c_void,
    ) -> VkResult<()> {
        ffi::vmaBindImageMemory2(
            self.handle,
            allocation_handle,
            allocation_local_offset,
            image,
            next,
        )
        .result()
    }

    /// Destroys Vulkan buffer and frees allocated memory.
    ///
    /// This is just a convenience function equivalent to:
    ///
    /// ```ignore
    /// ash::vk::Device::destroy_buffer(buffer, None);
    /// Allocator::free_memory(allocator, allocation);
    /// ```
    ///
    /// It it safe to pass null as `buffer` and/or `allocation`.
    pub unsafe fn vma_destroy_buffer(
        &self,
        buffer: ash::vk::Buffer,
        allocation_handle: ffi::VmaAllocation,
    ) {
        ffi::vmaDestroyBuffer(self.handle, buffer, allocation_handle);
    }

    /// Destroys Vulkan image and frees allocated memory.
    ///
    /// This is just a convenience function equivalent to:
    ///
    /// ```ignore
    /// ash::vk::Device::destroy_image(image, None);
    /// Allocator::free_memory(allocator, allocation);
    /// ```
    ///
    /// It it safe to pass null as `image` and/or `allocation`.
    pub unsafe fn vma_destroy_image(
        &self,
        image: ash::vk::Image,
        allocation_handle: ffi::VmaAllocation,
    ) {
        ffi::vmaDestroyImage(self.handle, image, allocation_handle);
    }
    /// Flushes memory of given set of allocations."]
    ///
    /// Calls `vkFlushMappedMemoryRanges()` for memory associated with given ranges of given allocations."]
    /// For more information, see documentation of vmaFlushAllocation()."]
    ///
    /// * `allocations`
    /// * `offsets` - If not None, it must be a slice of offsets of regions to flush, relative to the beginning of respective allocations. None means all ofsets are zero.
    /// * `sizes` - If not None, it must be a slice of sizes of regions to flush in respective allocations. None means `VK_WHOLE_SIZE` for all allocations.
    pub unsafe fn vma_flush_allocations<'a>(
        &self,
        allocation_handles: &mut [ffi::VmaAllocation],
        offsets: Option<&[vk::DeviceSize]>,
        sizes: Option<&[vk::DeviceSize]>,
    ) -> VkResult<()> {
        ffi::vmaFlushAllocations(
            self.handle,
            allocation_handles.len() as u32,
            allocation_handles.as_mut_ptr(),
            offsets.map_or(std::ptr::null(), |offsets| offsets.as_ptr()),
            sizes.map_or(std::ptr::null(), |sizes| sizes.as_ptr()),
        )
        .result()
    }

    /// Invalidates memory of given set of allocations."]
    ///
    /// Calls `vkInvalidateMappedMemoryRanges()` for memory associated with given ranges of given allocations."]
    /// For more information, see documentation of vmaInvalidateAllocation()."]
    ///
    /// * `allocations`
    /// * `offsets` - If not None, it must be a slice of offsets of regions to flush, relative to the beginning of respective allocations. None means all ofsets are zero.
    /// * `sizes` - If not None, it must be a slice of sizes of regions to flush in respective allocations. None means `VK_WHOLE_SIZE` for all allocations.
    pub unsafe fn vma_invalidate_allocations<'a>(
        &self,
        allocation_handles: &mut [ffi::VmaAllocation],
        offsets: Option<&[vk::DeviceSize]>,
        sizes: Option<&[vk::DeviceSize]>,
    ) -> VkResult<()> {
        ffi::vmaInvalidateAllocations(
            self.handle,
            allocation_handles.len() as u32,
            allocation_handles.as_mut_ptr(),
            offsets.map_or(std::ptr::null(), |offsets| offsets.as_ptr()),
            sizes.map_or(std::ptr::null(), |sizes| sizes.as_ptr()),
        )
        .result()
    }

    /// Begins defragmentation process.
    ///
    /// ## Returns
    /// `VK_SUCCESS` if defragmentation can begin.
    /// `VK_ERROR_FEATURE_NOT_PRESENT` if defragmentation is not supported.
    pub unsafe fn begin_defragmentation(
        &self,
        info: &ffi::VmaDefragmentationInfo,
    ) -> VkResult<DefragmentationContext> {
        let mut handle: ffi::VmaDefragmentationContext = std::ptr::null_mut();
        ffi::vmaBeginDefragmentation(self.handle, info, &mut handle).result()?;
        Ok(DefragmentationContext::new(handle, self))
    }

    // Getters

    /// Access the `bort_vma::Allocator` struct that `self` contains. Allows you to access vma allocator
    /// functions.
    #[inline]
    pub fn handle(&self) -> ffi::VmaAllocator {
        self.handle
    }
}

/// Custom `Drop` implementation to clean up internal allocation instance
impl Drop for MemoryAllocator {
    fn drop(&mut self) {
        unsafe {
            ffi::vmaDestroyAllocator(self.handle);
            self.handle = std::ptr::null_mut();
        }
    }
}

unsafe impl Send for MemoryAllocator {}
unsafe impl Sync for MemoryAllocator {}

impl AllocatorAccess for MemoryAllocator {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn memory_allocator(&self) -> &MemoryAllocator {
        &self
    }

    #[inline]
    fn memory_pool_handle(&self) -> ffi::VmaPool {
        std::ptr::null_mut()
    }
}
