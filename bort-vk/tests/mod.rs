extern crate ash;
extern crate bort_vk;
extern crate bort_vma;

use ash::{
    ext::debug_utils,
    vk::{self, EXT_DEBUG_UTILS_NAME},
};
use bort_vk::{
    ApiVersion, DebugCallback, DebugCallbackProperties, Device, Instance, PhysicalDevice,
};
use bort_vma::ffi;
use std::{
    ffi::{CStr, CString},
    os::raw::c_void,
    sync::Arc,
};

fn extension_names() -> Vec<*const i8> {
    vec![EXT_DEBUG_UTILS_NAME.as_ptr()]
}
const VALIDATION_LAYER_NAME: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") };

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
pub fn create_entry() -> Result<Arc<ash::Entry>, ash::LoadingError> {
    let entry = unsafe { ash::Entry::load() }?;
    Ok(Arc::new(entry))
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
pub fn create_entry() -> Result<Arc<ash::Entry>, ash::LoadingError> {
    let entry = ash_molten::load();
    Ok(Arc::new(entry))
}

unsafe extern "system" fn vulkan_debug_callback(
    _message_severity: ash::vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_types: ash::vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const ash::vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> ash::vk::Bool32 {
    let p_callback_data = &*p_callback_data;
    println!(
        "{:?}",
        ::std::ffi::CStr::from_ptr(p_callback_data.p_message)
    );
    ash::vk::FALSE
}

pub struct TestHarness {
    pub instance: Arc<Instance>,
    pub device: Arc<Device>,
    pub physical_device: Arc<PhysicalDevice>,
}

impl TestHarness {
    pub fn new() -> Self {
        let entry = create_entry().unwrap();

        let validation_layer_installed =
            Instance::layer_avilable(&entry, VALIDATION_LAYER_NAME.to_owned()).unwrap();
        let debug_utils_supported =
            Instance::supports_extension(&entry, None, EXT_DEBUG_UTILS_NAME.to_owned()).unwrap();

        let mut enable_validation = true;
        let mut instance_layers = Vec::<CString>::new();
        let mut instance_extensions = Vec::<CString>::new();
        if validation_layer_installed && debug_utils_supported {
            instance_layers.push(VALIDATION_LAYER_NAME.to_owned());
            instance_extensions.push(EXT_DEBUG_UTILS_NAME.to_owned());
        } else {
            enable_validation = false;
        }

        let instance = Arc::new(
            Instance::new(
                entry.clone(),
                ApiVersion { major: 1, minor: 3 },
                instance_layers,
                instance_extensions,
            )
            .unwrap(),
        );

        let debug_callback = if enable_validation {
            let debug_callback_properties = DebugCallbackProperties::default();
            let debug_callback = DebugCallback::new(
                instance.clone(),
                Some(vulkan_debug_callback),
                debug_callback_properties,
            )
            .unwrap();

            Some(Arc::new(debug_callback))
        } else {
            None
        };

        let physical_device_handles = instance.enumerate_physical_devices().unwrap();
        let physical_device_handle = physical_device_handles.first().unwrap();
        let physical_device =
            Arc::new(PhysicalDevice::new(instance.clone(), *physical_device_handle).unwrap());

        let queue_priorities = [1.0];
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(0)
            .queue_priorities(&queue_priorities);

        let device = Arc::new(
            Device::new(
                physical_device.clone(),
                [queue_create_info],
                Default::default(),
                Vec::new(),
                vec![],
                debug_callback,
            )
            .unwrap(),
        );

        TestHarness {
            instance,
            device,
            physical_device,
        }
    }

    pub fn create_allocator(&self) -> ffi::VmaAllocator {
        let create_info =
            bort_vma::AllocatorCreateInfo::new(&self.instance, &self.device, self.physical_device);
        unsafe { new_vma_allocator(create_info).unwrap() }
    }
}

#[test]
fn create_harness() {
    let _ = TestHarness::new();
}

#[test]
fn create_allocator() {
    let harness = TestHarness::new();
    let _ = harness.create_allocator();
}

#[test]
fn create_gpu_buffer() {
    let harness = TestHarness::new();
    let allocator = harness.create_allocator();
    let allocation_info = bort_vma::AllocationCreateInfo {
        usage: bort_vma::MemoryUsage::Auto,
        ..Default::default()
    };

    unsafe {
        let (buffer, mut allocation) = allocator
            .vma_create_buffer(
                &ash::vk::BufferCreateInfo::default().size(16 * 1024).usage(
                    ash::vk::BufferUsageFlags::VERTEX_BUFFER
                        | ash::vk::BufferUsageFlags::TRANSFER_DST,
                ),
                &allocation_info,
            )
            .unwrap();
        let allocation_info = allocator.get_allocation_info(&allocation);
        assert_eq!(allocation_info.mapped_data, std::ptr::null_mut());
        allocator.destroy_buffer(buffer, &mut allocation);
    }
}

#[test]
fn create_cpu_buffer_preferred() {
    let harness = TestHarness::new();
    let allocator = harness.create_allocator();
    let allocation_info = bort_vma::AllocationCreateInfo {
        required_flags: ash::vk::MemoryPropertyFlags::HOST_VISIBLE,
        preferred_flags: ash::vk::MemoryPropertyFlags::HOST_COHERENT
            | ash::vk::MemoryPropertyFlags::HOST_CACHED,
        flags: bort_vma::AllocationCreateFlags::MAPPED,
        ..Default::default()
    };
    unsafe {
        let (buffer, mut allocation) = allocator
            .create_buffer(
                &ash::vk::BufferCreateInfo::default().size(16 * 1024).usage(
                    ash::vk::BufferUsageFlags::VERTEX_BUFFER
                        | ash::vk::BufferUsageFlags::TRANSFER_DST,
                ),
                &allocation_info,
            )
            .unwrap();
        let allocation_info = allocator.get_allocation_info(&allocation);
        assert_ne!(allocation_info.mapped_data, std::ptr::null_mut());
        allocator.destroy_buffer(buffer, &mut allocation);
    }
}

#[test]
fn create_gpu_buffer_pool() {
    let harness = TestHarness::new();
    let allocator = harness.create_allocator();
    let allocator = Arc::new(allocator);

    let buffer_info = ash::vk::BufferCreateInfo::default()
        .size(16 * 1024)
        .usage(ash::vk::BufferUsageFlags::UNIFORM_BUFFER | ash::vk::BufferUsageFlags::TRANSFER_DST);

    let allocation_info = bort_vma::AllocationCreateInfo {
        required_flags: ash::vk::MemoryPropertyFlags::HOST_VISIBLE,
        preferred_flags: ash::vk::MemoryPropertyFlags::HOST_COHERENT
            | ash::vk::MemoryPropertyFlags::HOST_CACHED,
        flags: bort_vma::AllocationCreateFlags::MAPPED,

        ..Default::default()
    };
    unsafe {
        let memory_type_index = allocator
            .find_memory_type_index_for_buffer_info(&buffer_info, &allocation_info)
            .unwrap();

        // Create a pool that can have at most 2 blocks, 128 MiB each.
        let pool_info = bort_vma::PoolCreateInfo::new()
            .memory_type_index(memory_type_index)
            .block_size(128 * 1024 * 1024)
            .max_block_count(2);

        let pool = AllocatorPool::new(allocator.clone(), &pool_info).unwrap();

        let (buffer, mut allocation) = pool.create_buffer(&buffer_info, &allocation_info).unwrap();
        let allocation_info = allocator.get_allocation_info(&allocation);
        assert_ne!(allocation_info.mapped_data, std::ptr::null_mut());
        allocator.destroy_buffer(buffer, &mut allocation);
    }
}

#[test]
fn test_gpu_stats() {
    let harness = TestHarness::new();
    let allocator = harness.create_allocator();
    let allocation_info = bort_vma::AllocationCreateInfo {
        usage: bort_vma::MemoryUsage::Auto,
        ..Default::default()
    };

    unsafe {
        let stats_1 = allocator.calculate_statistics().unwrap();
        assert_eq!(stats_1.total.statistics.blockCount, 0);
        assert_eq!(stats_1.total.statistics.allocationCount, 0);
        assert_eq!(stats_1.total.statistics.allocationBytes, 0);

        let (buffer, mut allocation) = allocator
            .create_buffer(
                &ash::vk::BufferCreateInfo::default().size(16 * 1024).usage(
                    ash::vk::BufferUsageFlags::VERTEX_BUFFER
                        | ash::vk::BufferUsageFlags::TRANSFER_DST,
                ),
                &allocation_info,
            )
            .unwrap();

        let stats_2 = allocator.calculate_statistics().unwrap();
        assert_eq!(stats_2.total.statistics.blockCount, 1);
        assert_eq!(stats_2.total.statistics.allocationCount, 1);
        assert_eq!(stats_2.total.statistics.allocationBytes, 16 * 1024);

        allocator.destroy_buffer(buffer, &mut allocation);

        let stats_3 = allocator.calculate_statistics().unwrap();
        assert_eq!(stats_3.total.statistics.blockCount, 1);
        assert_eq!(stats_3.total.statistics.allocationCount, 0);
        assert_eq!(stats_3.total.statistics.allocationBytes, 0);
    }
}
