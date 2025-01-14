use crate::{device::Device, AllocatorAccess};
use ash::vk;
use bort_vma::{ffi, AllocationCreateInfo};
#[cfg(feature = "bytemuck")]
use bytemuck::{NoUninit, Pod, PodCastError};
use std::{error, fmt, mem, ptr, sync::Arc};

// ~~ Memory Allocation ~~

/// Represents single memory allocation.
///
/// It may be either dedicated block of `ash::vk::DeviceMemory` or a specific region of a
/// bigger block of this type plus unique offset.
///
/// Although the library provides convenience functions that create a Vulkan buffer or image,
/// allocate memory for it and bind them together, binding of the allocation to a buffer or an
/// image is out of scope of the allocation itself.
///
/// Allocation object can exist without buffer/image bound, binding can be done manually by
/// the user, and destruction of it can be done independently of destruction of the allocation.
///
/// The object also remembers its size and some other information. To retrieve this information,
/// use `Allocator::get_allocation_info`.
///
/// Some kinds allocations can be in lost state.
///
/// **Note:** this doesn't impl `Drop`. Destroy this yourself! e.g. with `Buffer` and `Image` `Drop` implementations
pub struct MemoryAllocation {
    handle: ffi::VmaAllocation,
    memory_type: vk::MemoryType,
    size: vk::DeviceSize,

    // dependencies
    allocator_access: Arc<dyn AllocatorAccess>,
}

impl MemoryAllocation {
    pub(crate) fn from_vma_allocation(
        handle: ffi::VmaAllocation,
        allocator_access: Arc<dyn AllocatorAccess>,
    ) -> Self {
        let memory_info = allocator_access
            .memory_allocator()
            .vma_get_allocation_info(handle);

        let size = memory_info.size;

        let physical_device_mem_props = allocator_access
            .device()
            .physical_device()
            .memory_properties();

        debug_assert!(memory_info.memory_type < physical_device_mem_props.memory_type_count);
        let memory_type = physical_device_mem_props.memory_types[memory_info.memory_type as usize];

        Self {
            handle,
            memory_type,
            size,
            allocator_access,
        }
    }

    #[cfg(feature = "bytemuck")]
    pub fn write_into_bytes<T>(
        &mut self,
        write_data: T,
        allocation_offset: usize,
    ) -> Result<(), MemoryError>
    where
        T: NoUninit,
    {
        let write_bytes = bytemuck::bytes_of(&write_data);
        self.write_bytes(write_bytes, allocation_offset)
    }

    #[cfg(feature = "bytemuck")]
    pub fn write_slice<T>(
        &mut self,
        write_data: &[T],
        allocation_offset: usize,
    ) -> Result<(), MemoryError>
    where
        T: NoUninit,
    {
        let write_bytes: &[u8] = bytemuck::try_cast_slice(write_data)?;
        self.write_bytes(write_bytes, allocation_offset)
    }

    pub fn write_bytes(
        &mut self,
        write_bytes: &[u8],
        allocation_offset: usize,
    ) -> Result<(), MemoryError> {
        let data_size = write_bytes.len();
        self.check_memory_access_parameters(data_size, allocation_offset)?;

        let offset_mapped_memory: *mut u8 =
            unsafe { self.map_memory_with_offset_unchecked(allocation_offset)? };

        let write_bytes_ptr = write_bytes.as_ptr();
        unsafe {
            ptr::copy_nonoverlapping(write_bytes_ptr, offset_mapped_memory, data_size);
        }

        let flush_res = self.flush_allocation(allocation_offset, data_size);
        unsafe { self.unmap_memory() };
        flush_res
    }

    /// Writes `data` to this memory allocation. Will flush if memory isn't host coherent.
    ///
    /// If memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` this will fail.
    pub fn write_struct<T>(
        &mut self,
        write_data: T,
        allocation_offset: usize,
    ) -> Result<(), MemoryError> {
        let data_size = mem::size_of_val(&write_data);
        self.check_memory_access_parameters(data_size, allocation_offset)?;

        let offset_mapped_memory: *mut T =
            unsafe { self.map_memory_with_offset_unchecked(allocation_offset)? };

        unsafe { ptr::write::<T>(offset_mapped_memory, write_data) };

        let flush_res = self.flush_allocation(allocation_offset, data_size);
        unsafe { self.unmap_memory() };
        flush_res
    }

    /// _WARNING:_ This function writes one element at a time. In comparison, the `write_slice`
    /// function will copy everything in one go (requires the `bytemuck` feature enabled).
    ///
    /// If memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` this will fail.
    pub fn write_iter<I, T>(
        &mut self,
        write_data: I,
        allocation_offset: usize,
    ) -> Result<(), MemoryError>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        let write_data_iter = write_data.into_iter();
        let item_size = mem::size_of::<T>();
        let data_size = write_data_iter.len() * item_size;
        self.check_memory_access_parameters(data_size, allocation_offset)?;

        let mut offset_mapped_memory: *mut T =
            unsafe { self.map_memory_with_offset_unchecked(allocation_offset)? };

        for element in write_data_iter {
            unsafe {
                ptr::write::<T>(offset_mapped_memory, element);
                offset_mapped_memory = offset_mapped_memory.offset(1);
            }
        }

        let flush_res = self.flush_allocation(allocation_offset, data_size);
        unsafe { self.unmap_memory() };
        flush_res
    }

    #[cfg(feature = "bytemuck")]
    pub fn read_vec<T>(
        &mut self,
        element_count: usize,
        allocation_offset: usize,
    ) -> Result<Vec<T>, MemoryError>
    where
        T: Pod,
    {
        let type_size = mem::size_of::<T>();
        let data_size = type_size * element_count;
        self.check_memory_access_parameters(data_size, allocation_offset)?;

        let offset_mapped_memory: *mut T =
            unsafe { self.map_memory_with_offset_unchecked(allocation_offset)? };

        let mut output_vec = Vec::<T>::new();
        output_vec.resize_with(element_count, T::zeroed);
        let output_vec_ptr = output_vec.as_mut_ptr();

        unsafe { ptr::copy_nonoverlapping(offset_mapped_memory, output_vec_ptr, element_count) };

        unsafe { self.unmap_memory() };
        Ok(output_vec)
    }

    /// Writes `data` to this memory allocation. Will flush if memory isn't host coherent.
    ///
    /// If memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` this will fail.
    pub fn read_struct<T>(&mut self, allocation_offset: usize) -> Result<T, MemoryError> {
        let data_size = mem::size_of::<T>();
        self.check_memory_access_parameters(data_size, allocation_offset)?;

        let offset_mapped_memory: *mut T =
            unsafe { self.map_memory_with_offset_unchecked(allocation_offset)? };

        let read_data = unsafe { ptr::read::<T>(offset_mapped_memory) };

        unsafe { self.unmap_memory() };
        Ok(read_data)
    }

    fn check_memory_access_parameters(
        &self,
        data_size: usize,
        allocation_offset: usize,
    ) -> Result<(), MemoryError> {
        let allocation_size = self.size as usize; // allocation size won't be anywhere near the max of usize/isize despite being u64
        let allocation_write_size = allocation_size.checked_sub(allocation_offset).ok_or(
            MemoryError::AllocationOffsetTooBig {
                allocation_size: self.size,
                allocation_offset,
            },
        )?;

        if data_size > allocation_write_size {
            return Err(MemoryError::DataSizeTooBig {
                data_size,
                allocation_size: self.size,
                allocation_offset,
            });
        }

        Ok(())
    }

    /// # Safety
    /// See docs for [`bort_vma::allocator::Allocator::map_memory`]
    pub unsafe fn map_memory(&mut self) -> Result<*mut u8, MemoryError> {
        self.allocator_access
            .memory_allocator()
            .vma_map_memory(self.handle)
            .map_err(MemoryError::Mapping)
    }

    /// # Safety
    /// See docs for [`bort_vma::allocator::Allocator::unmap_memory`]
    pub unsafe fn unmap_memory(&mut self) {
        self.allocator_access
            .memory_allocator()
            .vma_unmap_memory(self.handle)
    }

    /// # Safety
    /// - `allocation_offset` must be within the allocation memory size
    /// - See docs for [`bort_vma::allocator::Allocator::map_memory`]
    unsafe fn map_memory_with_offset_unchecked<T>(
        &mut self,
        allocation_offset: usize,
    ) -> Result<*mut T, MemoryError> {
        let mapped_memory = unsafe { self.map_memory() }?;
        #[allow(clippy::ptr_offset_with_cast)] // want the interface to be unsigned because negative
        // offsets don't make sense when offsetting from the start of the allocation and if the offset
        // is peeking into the highest `usize` bit it would have been caught by prior checking functions
        let offset_mapped_memory =
            unsafe { mapped_memory.offset(allocation_offset as isize) } as *mut T;
        Ok(offset_mapped_memory)
    }

    /// Flushes allocated memory. Note that the VMA function only runs is the memory is host
    /// visible and isn't host coherent.
    #[inline]
    pub fn flush_allocation(
        &mut self,
        allocation_offset: usize,
        data_size: usize,
    ) -> Result<(), MemoryError> {
        self.allocator_access
            .memory_allocator()
            .vma_flush_allocation(self.handle, allocation_offset, data_size)
            .map_err(MemoryError::Flushing)
    }

    // Getters

    /// Access the `bort_vma::Allocation` handle that `self` contains.
    #[inline]
    pub fn handle(&self) -> ffi::VmaAllocation {
        self.handle
    }

    #[inline]
    pub fn memory_type(&self) -> vk::MemoryType {
        self.memory_type
    }

    /// Returns self as a dynamic allocation type.
    #[inline]
    pub fn allocator_access(&self) -> &Arc<dyn AllocatorAccess> {
        &self.allocator_access
    }

    #[inline]
    pub fn memory_property_flags(&self) -> vk::MemoryPropertyFlags {
        self.memory_type.property_flags
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        self.allocator_access.device()
    }
}

// ~~ Create Info ~~

/// Parameters of `Allocation` objects, that can be retrieved using `Allocator::get_allocation_info`.
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    /// Memory type index that this allocation was allocated from. It never changes.
    pub memory_type: u32,
    /// Handle to Vulkan memory object.
    ///
    /// Same memory object can be shared by multiple allocations.
    ///
    /// It can change after the allocation is moved during \\ref defragmentation.
    pub device_memory: vk::DeviceMemory,
    /// Offset in `VkDeviceMemory` object to the beginning of this allocation, in bytes. `(deviceMemory, offset)` pair is unique to this allocation.
    ///
    /// You usually don't need to use this offset. If you create a buffer or an image together with the allocation using e.g. function
    /// vmaCreateBuffer(), vmaCreateImage(), functions that operate on these resources refer to the beginning of the buffer or image,
    /// not entire device memory block. Functions like vmaMapMemory(), vmaBindBufferMemory() also refer to the beginning of the allocation
    /// and apply this offset automatically.
    ///
    /// It can change after the allocation is moved during \\ref defragmentation.
    pub offset: vk::DeviceSize,
    /// Size of this allocation, in bytes. It never changes.
    ///
    /// Allocation size returned in this variable may be greater than the size
    /// requested for the resource e.g. as `VkBufferCreateInfo::size`. Whole size of the
    /// allocation is accessible for operations on memory e.g. using a pointer after
    /// mapping with vmaMapMemory(), but operations on the resource e.g. using
    /// `vkCmdCopyBuffer` must be limited to the size of the resource.
    pub size: vk::DeviceSize,
    /// Pointer to the beginning of this allocation as mapped data.
    ///
    /// If the allocation hasn't been mapped using vmaMapMemory() and hasn't been
    /// created with #VMA_ALLOCATION_CREATE_MAPPED_BIT flag, this value is null.
    ///
    /// It can change after call to vmaMapMemory(), vmaUnmapMemory().
    /// It can also change after the allocation is moved during defragmentation.
    pub mapped_data: *mut ::std::os::raw::c_void,
    /// Custom general-purpose pointer that was passed as VmaAllocationCreateInfo::pUserData or set using vmaSetAllocationUserData().
    ///
    /// It can change after call to vmaSetAllocationUserData() for this allocation.
    pub user_data: usize,
}

impl From<&ffi::VmaAllocationInfo> for AllocationInfo {
    fn from(info: &ffi::VmaAllocationInfo) -> Self {
        Self {
            memory_type: info.memoryType,
            device_memory: info.deviceMemory,
            offset: info.offset,
            size: info.size,
            mapped_data: info.pMappedData,
            user_data: info.pUserData as _,
        }
    }
}
impl From<ffi::VmaAllocationInfo> for AllocationInfo {
    fn from(info: ffi::VmaAllocationInfo) -> Self {
        (&info).into()
    }
}

unsafe impl Send for MemoryAllocation {}
unsafe impl Sync for MemoryAllocation {}

// ~~ Presets ~~

/// Default `AllocationCreateInfo` with specified required and preferred flags.
pub fn allocation_info_from_flags(
    required_flags: vk::MemoryPropertyFlags,
    preferred_flags: vk::MemoryPropertyFlags,
) -> AllocationCreateInfo {
    AllocationCreateInfo {
        required_flags,
        preferred_flags,
        ..Default::default()
    }
}

/// For allocating memory that can be accessed and mapped from the cpu. Prefered flags include
/// HOST_COHERENT (doesn't require flushing). Good for staging buffers.
pub fn allocation_info_cpu_accessible() -> AllocationCreateInfo {
    // more info here: https://asawicki.info/news_1740_vulkan_memory_types_on_pc_and_how_to_use_them
    allocation_info_from_flags(
        vk::MemoryPropertyFlags::HOST_VISIBLE,
        vk::MemoryPropertyFlags::HOST_COHERENT,
    )
}

// ~~ Memory Error ~~

#[derive(Debug, Clone)]
pub enum MemoryError {
    Mapping(vk::Result),
    DataSizeTooBig {
        data_size: usize,
        allocation_size: vk::DeviceSize,
        allocation_offset: usize,
    },
    AllocationOffsetTooBig {
        allocation_size: vk::DeviceSize,
        allocation_offset: usize,
    },
    Flushing(vk::Result),
    #[cfg(feature = "bytemuck")]
    PodCastError(PodCastError),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mapping(e) => write!(f, "failed map allocation memory: {}", e),
            Self::DataSizeTooBig {
                data_size,
                allocation_size,
                allocation_offset,
            } => write!(
                f,
                "invalid data size access parameters: data size = {}, allocation size = {}, write offset = {}",
                data_size, allocation_size, allocation_offset
            ),
            Self::AllocationOffsetTooBig {
                allocation_size,
                allocation_offset,
            } => write!(f,
                "allocation offset {} is larger than the allocated memory {}",
                allocation_offset, allocation_size
            ),
            Self::Flushing(e) => write!(f, "failed to flush memory: {}", e),
            #[cfg(feature = "bytemuck")]
            Self::PodCastError(e) => write!(f, "slice cast failed: {}", e),
        }
    }
}

impl error::Error for MemoryError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Mapping(e) => Some(e),
            Self::DataSizeTooBig { .. } => None,
            Self::AllocationOffsetTooBig { .. } => None,
            Self::Flushing(e) => Some(e),
            #[cfg(feature = "bytemuck")]
            Self::PodCastError(e) => Some(e),
        }
    }
}

#[cfg(feature = "bytemuck")]
impl From<PodCastError> for MemoryError {
    fn from(e: PodCastError) -> Self {
        Self::PodCastError(e)
    }
}
