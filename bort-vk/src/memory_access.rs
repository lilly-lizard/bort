use crate::{Device, MemoryAllocation, MemoryAllocator, MemoryError};
use ash::{prelude::VkResult, vk};
use bort_vma::{ffi, AllocationCreateInfo};
#[cfg(feature = "bytemuck")]
use bytemuck::{NoUninit, Pod};
use std::sync::Arc;

// ~~ Allocator Access ~~

/// Unifies different types of vma allocators
pub trait AllocatorAccess: Send + Sync {
    fn device(&self) -> &Arc<Device>;
    fn memory_allocator(&self) -> &MemoryAllocator;
    fn memory_pool_handle(&self) -> ffi::VmaPool;

    /// Helps to find memory type index, given memory type bits and allocation info.
    ///
    /// This algorithm tries to find a memory type that:
    ///
    /// - Is allowed by memory type bits.
    /// - Contains all the flags from `allocation_info.required_flags`.
    /// - Matches intended usage.
    /// - Has as many flags from `allocation_info.preferred_flags` as possible.
    ///
    /// Returns ash::vk::Result::ERROR_FEATURE_NOT_PRESENT if not found. Receiving such a result
    /// from this function or any other allocating function probably means that your
    /// device doesn't support any memory type with requested features for the specific
    /// type of resource you want to use it for. Please check parameters of your
    /// resource, like image layout (OPTIMAL versus LINEAR) or mip level count.
    unsafe fn find_memory_type_index(
        &self,
        memory_type_bits: u32,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<u32> {
        let mut memory_type_index: u32 = 0;
        let mut allocation_info: ffi::VmaAllocationCreateInfo = allocation_info.into();
        allocation_info.pool = self.memory_pool_handle();
        ffi::vmaFindMemoryTypeIndex(
            self.memory_allocator().handle(),
            memory_type_bits,
            &allocation_info,
            &mut memory_type_index,
        )
        .result()?;

        Ok(memory_type_index)
    }

    /// Helps to find memory type index, given buffer info and allocation info.
    ///
    /// It can be useful e.g. to determine value to be used as `AllocatorPoolCreateInfo::memory_type_index`.
    /// It internally creates a temporary, dummy buffer that never has memory bound.
    /// It is just a convenience function, equivalent to calling:
    ///
    /// - `ash::vk::Device::create_buffer`
    /// - `ash::vk::Device::get_buffer_memory_requirements`
    /// - `Allocator::find_memory_type_index`
    /// - `ash::vk::Device::destroy_buffer`
    unsafe fn find_memory_type_index_for_buffer_info(
        &self,
        buffer_info: &ash::vk::BufferCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<u32> {
        let mut allocation_info: ffi::VmaAllocationCreateInfo = allocation_info.into();
        allocation_info.pool = self.memory_pool_handle();
        let mut memory_type_index: u32 = 0;
        ffi::vmaFindMemoryTypeIndexForBufferInfo(
            self.memory_allocator().handle(),
            buffer_info,
            &allocation_info,
            &mut memory_type_index,
        )
        .result()?;

        Ok(memory_type_index)
    }

    /// Helps to find memory type index, given image info and allocation info.
    ///
    /// It can be useful e.g. to determine value to be used as `AllocatorPoolCreateInfo::memory_type_index`.
    /// It internally creates a temporary, dummy image that never has memory bound.
    /// It is just a convenience function, equivalent to calling:
    ///
    /// - `ash::vk::Device::create_image`
    /// - `ash::vk::Device::get_image_memory_requirements`
    /// - `Allocator::find_memory_type_index`
    /// - `ash::vk::Device::destroy_image`
    unsafe fn find_memory_type_index_for_image_info(
        &self,
        image_info: ash::vk::ImageCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> VkResult<u32> {
        let mut allocation_info: ffi::VmaAllocationCreateInfo = allocation_info.into();
        allocation_info.pool = self.memory_pool_handle();
        let mut memory_type_index: u32 = 0;
        ffi::vmaFindMemoryTypeIndexForImageInfo(
            self.memory_allocator().handle(),
            &image_info,
            &allocation_info,
            &mut memory_type_index,
        )
        .result()?;

        Ok(memory_type_index)
    }

    /// General purpose memory allocation.
    ///
    /// You should free the memory using `Allocator::free_memory` or 'Allocator::free_memory_pages'.
    ///
    /// It is recommended to use `Allocator::allocate_memory_for_buffer`, `Allocator::allocate_memory_for_image`,
    /// `Allocator::create_buffer`, `Allocator::create_image` instead whenever possible.
    unsafe fn vma_allocate_memory(
        &self,
        memory_requirements: &ash::vk::MemoryRequirements,
        create_info: &AllocationCreateInfo,
    ) -> VkResult<ffi::VmaAllocation> {
        let mut create_info: ffi::VmaAllocationCreateInfo = create_info.into();
        create_info.pool = self.memory_pool_handle();
        let mut allocation_handle: ffi::VmaAllocation = std::mem::zeroed();
        ffi::vmaAllocateMemory(
            self.memory_allocator().handle(),
            memory_requirements,
            &create_info,
            &mut allocation_handle,
            std::ptr::null_mut(),
        )
        .result()?;

        Ok(allocation_handle)
    }

    /// General purpose memory allocation for multiple allocation objects at once.
    ///
    /// You should free the memory using `Allocator::free_memory` or `Allocator::free_memory_pages`.
    ///
    /// Word "pages" is just a suggestion to use this function to allocate pieces of memory needed for sparse binding.
    /// It is just a general purpose allocation function able to make multiple allocations at once.
    /// It may be internally optimized to be more efficient than calling `Allocator::allocate_memory` `allocations.len()` times.
    ///
    /// All allocations are made using same parameters. All of them are created out of the same memory pool and type.
    unsafe fn vma_allocate_memory_pages(
        &self,
        memory_requirements: &ash::vk::MemoryRequirements,
        create_info: &AllocationCreateInfo,
        allocation_count: usize,
    ) -> VkResult<Vec<ffi::VmaAllocation>> {
        let mut create_info: ffi::VmaAllocationCreateInfo = create_info.into();
        create_info.pool = self.memory_pool_handle();
        let mut allocation_handles: Vec<ffi::VmaAllocation> =
            vec![std::mem::zeroed(); allocation_count];
        ffi::vmaAllocateMemoryPages(
            self.memory_allocator().handle(),
            memory_requirements,
            &create_info,
            allocation_count,
            allocation_handles.as_mut_ptr(),
            std::ptr::null_mut(),
        )
        .result()?;

        Ok(allocation_handles)
    }

    /// Buffer specialized memory allocation.
    ///
    /// You should free the memory using `Allocator::free_memory` or 'Allocator::free_memory_pages'.
    unsafe fn vma_allocate_memory_for_buffer(
        &self,
        buffer: ash::vk::Buffer,
        create_info: &AllocationCreateInfo,
    ) -> VkResult<ffi::VmaAllocation> {
        let mut create_info: ffi::VmaAllocationCreateInfo = create_info.into();
        create_info.pool = self.memory_pool_handle();
        let mut allocation_handle: ffi::VmaAllocation = std::mem::zeroed();
        let mut allocation_info: ffi::VmaAllocationInfo = std::mem::zeroed();
        ffi::vmaAllocateMemoryForBuffer(
            self.memory_allocator().handle(),
            buffer,
            &create_info,
            &mut allocation_handle,
            &mut allocation_info,
        )
        .result()?;

        Ok(allocation_handle)
    }

    /// Image specialized memory allocation.
    ///
    /// You should free the memory using `Allocator::free_memory` or 'Allocator::free_memory_pages'.
    unsafe fn vma_allocate_memory_for_image(
        &self,
        image: ash::vk::Image,
        create_info: &AllocationCreateInfo,
    ) -> VkResult<ffi::VmaAllocation> {
        let mut create_info: ffi::VmaAllocationCreateInfo = create_info.into();
        create_info.pool = self.memory_pool_handle();
        let mut allocation_handle: ffi::VmaAllocation = std::mem::zeroed();
        ffi::vmaAllocateMemoryForImage(
            self.memory_allocator().handle(),
            image,
            &create_info,
            &mut allocation_handle,
            std::ptr::null_mut(),
        )
        .result()?;

        Ok(allocation_handle)
    }

    /// This function automatically creates a buffer, allocates appropriate memory
    /// for it, and binds the buffer with the memory.
    ///
    /// If the function succeeded, you must destroy both buffer and allocation when you
    /// no longer need them using either convenience function `Allocator::destroy_buffer` or
    /// separately, using `ash::Device::destroy_buffer` and `Allocator::free_memory`.
    ///
    /// If `AllocatorCreateFlags::KHR_DEDICATED_ALLOCATION` flag was used,
    /// VK_KHR_dedicated_allocation extension is used internally to query driver whether
    /// it requires or prefers the new buffer to have dedicated allocation. If yes,
    /// and if dedicated allocation is possible (AllocationCreateInfo::pool is null
    /// and `AllocationCreateFlags::NEVER_ALLOCATE` is not used), it creates dedicated
    /// allocation for this buffer, just like when using `AllocationCreateFlags::DEDICATED_MEMORY`.
    unsafe fn vma_create_buffer(
        &self,
        buffer_info: &ash::vk::BufferCreateInfo,
        create_info: &AllocationCreateInfo,
    ) -> VkResult<(ash::vk::Buffer, ffi::VmaAllocation)> {
        let mut create_info: ffi::VmaAllocationCreateInfo = create_info.into();
        create_info.pool = self.memory_pool_handle();
        let mut buffer = vk::Buffer::null();
        let mut allocation_handle: ffi::VmaAllocation = std::mem::zeroed();
        ffi::vmaCreateBuffer(
            self.memory_allocator().handle(),
            buffer_info,
            &create_info,
            &mut buffer,
            &mut allocation_handle,
            std::ptr::null_mut(),
        )
        .result()?;

        Ok((buffer, allocation_handle))
    }

    /// brief Creates a buffer with additional minimum alignment.
    ///
    /// Similar to vmaCreateBuffer() but provides additional parameter `minAlignment` which allows to specify custom,
    /// minimum alignment to be used when placing the buffer inside a larger memory block, which may be needed e.g.
    /// for interop with OpenGL.
    unsafe fn vma_create_buffer_with_alignment(
        &self,
        buffer_info: &ash::vk::BufferCreateInfo,
        create_info: &AllocationCreateInfo,
        min_alignment: vk::DeviceSize,
    ) -> VkResult<(ash::vk::Buffer, ffi::VmaAllocation)> {
        let mut create_info: ffi::VmaAllocationCreateInfo = create_info.into();
        create_info.pool = self.memory_pool_handle();
        let mut buffer = vk::Buffer::null();
        let mut allocation_handle: ffi::VmaAllocation = std::mem::zeroed();
        ffi::vmaCreateBufferWithAlignment(
            self.memory_allocator().handle(),
            buffer_info,
            &create_info,
            min_alignment,
            &mut buffer,
            &mut allocation_handle,
            std::ptr::null_mut(),
        )
        .result()?;

        Ok((buffer, allocation_handle))
    }

    /// This function automatically creates an image, allocates appropriate memory
    /// for it, and binds the image with the memory.
    ///
    /// If the function succeeded, you must destroy both image and allocation when you
    /// no longer need them using either convenience function `Allocator::destroy_image` or
    /// separately, using `ash::Device::destroy_image` and `Allocator::free_memory`.
    ///
    /// If `AllocatorCreateFlags::KHR_DEDICATED_ALLOCATION` flag was used,
    /// `VK_KHR_dedicated_allocation extension` is used internally to query driver whether
    /// it requires or prefers the new image to have dedicated allocation. If yes,
    /// and if dedicated allocation is possible (AllocationCreateInfo::pool is null
    /// and `AllocationCreateFlags::NEVER_ALLOCATE` is not used), it creates dedicated
    /// allocation for this image, just like when using `AllocationCreateFlags::DEDICATED_MEMORY`.
    ///
    /// If `VK_ERROR_VALIDAITON_FAILED_EXT` is returned, VMA may have encountered a problem
    /// that is not caught by the validation layers. One example is if you try to create a 0x0
    /// image, a panic will occur and `VK_ERROR_VALIDAITON_FAILED_EXT` is thrown.
    unsafe fn vma_create_image(
        &self,
        image_info: &ash::vk::ImageCreateInfo,
        create_info: &AllocationCreateInfo,
    ) -> VkResult<(ash::vk::Image, ffi::VmaAllocation)> {
        let mut create_info: ffi::VmaAllocationCreateInfo = create_info.into();
        create_info.pool = self.memory_pool_handle();
        let mut image = vk::Image::null();
        let mut allocation_handle: ffi::VmaAllocation = std::mem::zeroed();
        ffi::vmaCreateImage(
            self.memory_allocator().handle(),
            image_info,
            &create_info,
            &mut image,
            &mut allocation_handle,
            std::ptr::null_mut(),
        )
        .result()?;

        Ok((image, allocation_handle))
    }
}

// ~~ MemoryAllocation Access ~~

/// Allows any struct containing a memory allocation to "inherit" the read/write functions
pub trait AllocationAccess {
    fn memory_allocation_mut(&mut self) -> &mut MemoryAllocation;

    #[cfg(feature = "bytemuck")]
    fn write_into_bytes<T>(
        &mut self,
        write_data: T,
        allocation_offset: usize,
    ) -> Result<(), MemoryError>
    where
        T: NoUninit,
    {
        self.memory_allocation_mut()
            .write_into_bytes(write_data, allocation_offset)
    }

    #[cfg(feature = "bytemuck")]
    fn write_slice<T>(
        &mut self,
        write_data: &[T],
        allocation_offset: usize,
    ) -> Result<(), MemoryError>
    where
        T: NoUninit,
    {
        self.memory_allocation_mut()
            .write_slice(write_data, allocation_offset)
    }

    fn write_bytes(
        &mut self,
        write_bytes: &[u8],
        allocation_offset: usize,
    ) -> Result<(), MemoryError> {
        self.memory_allocation_mut()
            .write_bytes(write_bytes, allocation_offset)
    }

    fn write_struct<T>(
        &mut self,
        write_data: T,
        allocation_offset: usize,
    ) -> Result<(), MemoryError> {
        self.memory_allocation_mut()
            .write_struct(write_data, allocation_offset)
    }

    fn write_iter<I, T>(
        &mut self,
        write_data: I,
        allocation_offset: usize,
    ) -> Result<(), MemoryError>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.memory_allocation_mut()
            .write_iter(write_data, allocation_offset)
    }

    #[cfg(feature = "bytemuck")]
    fn read_vec<T>(
        &mut self,
        element_count: usize,
        allocation_offset: usize,
    ) -> Result<Vec<T>, MemoryError>
    where
        T: Pod,
    {
        self.memory_allocation_mut()
            .read_vec(element_count, allocation_offset)
    }

    fn read_struct<T>(&mut self, allocation_offset: usize) -> Result<T, MemoryError> {
        self.memory_allocation_mut().read_struct(allocation_offset)
    }
}
