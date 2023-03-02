use crate::device::Device;
use ash::{prelude::VkResult, vk};
use std::{error, fmt, mem, ptr, sync::Arc};
use vk_mem::{AllocationCreateInfo, AllocatorCreateInfo};

/// so it's easy to find all allocation callback args, just in case I want to use them in the future.
pub const ALLOCATION_CALLBACK_NONE: Option<&ash::vk::AllocationCallbacks> = None;

// Memory Allocator

pub struct MemoryAllocator {
    inner: vk_mem::Allocator,

    // dependencies
    device: Arc<Device>,
}

impl MemoryAllocator {
    pub fn new(device: Arc<Device>) -> VkResult<Self> {
        let allocator_info = AllocatorCreateInfo::new(
            device.instance().inner(),
            device.inner(),
            device.physical_device().handle(),
        );
        let inner = vk_mem::Allocator::new(allocator_info)?;

        Ok(Self { inner, device })
    }

    // Getters

    pub fn inner(&self) -> &vk_mem::Allocator {
        &self.inner
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

// Memory Allocation

/// Note this doesn't impl `Drop`. Destroy this yourself! (See `Buffer` and `Image`)
pub struct MemoryAllocation {
    inner: vk_mem::Allocation,
    memory_type: vk::MemoryType,
    size: vk::DeviceSize,

    // dependencies
    memory_allocator: Arc<MemoryAllocator>,
}

impl MemoryAllocation {
    pub(crate) fn from_vma_allocation(
        inner: vk_mem::Allocation,
        memory_allocator: Arc<MemoryAllocator>,
    ) -> Self {
        let memory_info = memory_allocator.inner().get_allocation_info(&inner);

        let size = memory_info.size;

        let physical_device_mem_props = memory_allocator
            .device()
            .physical_device()
            .memory_properties();

        debug_assert!(memory_info.memory_type < physical_device_mem_props.memory_type_count);
        let memory_type = physical_device_mem_props.memory_types[memory_info.memory_type as usize];

        Self {
            inner,
            memory_type,
            size,
            memory_allocator,
        }
    }

    /// Note that if memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` writing will fail
    pub fn write_struct<T>(&mut self, data: T, write_offset: usize) -> Result<(), MemoryError> {
        let data_size = mem::size_of_val(&data);

        let allocation_size = self.size as usize;
        if data_size > allocation_size - write_offset {
            return Err(MemoryError::WriteDataSize {
                data_size,
                allocation_size,
                write_offset,
            });
        }

        let mapped_memory = unsafe { self.map_memory() }?;
        let offset_mapped_memory = unsafe { mapped_memory.offset(write_offset as isize) } as *mut T;

        unsafe { ptr::write::<T>(offset_mapped_memory, data) };

        let flush_res = self.flush_allocation(write_offset, data_size);
        if let Err(e) = flush_res {
            unsafe { self.unmap_memory() };
            return Err(e);
        }

        unsafe { self.unmap_memory() };

        Ok(())
    }

    /// Note that if memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` writing will fail
    pub fn write_iter<I, T>(&mut self, data: I, write_offset: usize) -> Result<(), MemoryError>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        let data = data.into_iter();
        let item_size = mem::size_of::<T>();
        let data_size = data.len() * item_size;

        let allocation_size = self.size as usize;
        if data_size > allocation_size - write_offset {
            return Err(MemoryError::WriteDataSize {
                data_size,
                allocation_size,
                write_offset,
            });
        }

        let mapped_memory = unsafe { self.map_memory() }?;
        let mut offset_mapped_memory =
            unsafe { mapped_memory.offset(write_offset as isize) } as *mut T;

        unsafe {
            for element in data {
                ptr::write::<T>(offset_mapped_memory, element);
                offset_mapped_memory = offset_mapped_memory.offset(1);
            }
        }

        let flush_res = self.flush_allocation(write_offset, data_size);
        if let Err(e) = flush_res {
            unsafe { self.unmap_memory() };
            return Err(e);
        }

        unsafe { self.unmap_memory() };

        Ok(())
    }

    pub unsafe fn map_memory(&mut self) -> Result<*mut u8, MemoryError> {
        self.memory_allocator
            .inner()
            .map_memory(&mut self.inner)
            .map_err(|e| MemoryError::Mapping(e))
    }

    pub unsafe fn unmap_memory(&mut self) {
        self.memory_allocator.inner().unmap_memory(&mut self.inner)
    }

    pub fn flush_allocation(
        &mut self,
        write_offset: usize,
        data_size: usize,
    ) -> Result<(), MemoryError> {
        // don't need to flush if memory is host coherent
        if !self
            .memory_property_flags()
            .contains(vk::MemoryPropertyFlags::HOST_COHERENT)
        {
            self.memory_allocator
                .inner()
                .flush_allocation(&self.inner, write_offset, data_size)
                .map_err(|e| MemoryError::Flushing(e))?;
        }

        Ok(())
    }

    // Getters

    #[inline]
    pub fn inner(&self) -> &vk_mem::Allocation {
        &self.inner
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut vk_mem::Allocation {
        &mut self.inner
    }

    #[inline]
    pub fn memory_type(&self) -> vk::MemoryType {
        self.memory_type
    }

    #[inline]
    pub fn memory_allocator(&self) -> &Arc<MemoryAllocator> {
        &self.memory_allocator
    }

    #[inline]
    pub fn memory_property_flags(&self) -> vk::MemoryPropertyFlags {
        self.memory_type.property_flags
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.memory_allocator.device()
    }
}

// Presets

/// For allocating memory that can be accessed and mapped from the cpu. Prefered flags include
/// memory that is host coherent (doesn't require flushing) and device local (fast gpu access)
pub fn cpu_accessible_allocation_info() -> AllocationCreateInfo {
    AllocationCreateInfo {
        flags: vk_mem::AllocationCreateFlags::HOST_ACCESS_RANDOM,
        required_flags: vk::MemoryPropertyFlags::HOST_VISIBLE,
        preferred_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL
            | vk::MemoryPropertyFlags::HOST_COHERENT,
        ..Default::default()
    }
}

// Helper Functions

pub fn find_memorytype_index(
    memory_req: &vk::MemoryRequirements,
    memory_prop: &vk::PhysicalDeviceMemoryProperties,
    flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    memory_prop.memory_types[..memory_prop.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1 << index) & memory_req.memory_type_bits != 0
                && memory_type.property_flags & flags == flags
        })
        .map(|(index, _memory_type)| index as _)
}

#[derive(Debug, Clone)]
pub enum MemoryError {
    Mapping(vk::Result),
    WriteDataSize {
        data_size: usize,
        allocation_size: usize,
        write_offset: usize,
    },
    Flushing(vk::Result),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mapping(e) => {
                write!(f, "failed map allocation memory: {}", e)
            }
            Self::WriteDataSize {
                data_size,
                allocation_size,
                write_offset,
            } => {
                write!(
                    f,
                    "invalid data size to be written: data size = {}, allocation size = {}, write offset = {}",
                    data_size, allocation_size, write_offset
                )
            }
            Self::Flushing(e) => write!(f, "failed to flush memory: {}", e),
        }
    }
}

impl error::Error for MemoryError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Mapping(e) => Some(e),
            Self::WriteDataSize { .. } => None,
            Self::Flushing(e) => Some(e),
        }
    }
}
