use ash::vk;
use bort_vk::{
    choose_composite_alpha, is_format_srgb, ApiVersion, CommandPool, CommandPoolProperties, Device,
    ImageView, Instance, MemoryAllocator, PhysicalDevice, Queue, Surface, Swapchain,
    SwapchainProperties,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{error::Error, sync::Arc};
use winit::{event_loop::EventLoop, window::WindowBuilder};

const TITLE: &str = "Triangle (bort example)";
const DEFAULT_WINDOW_SIZE: [u32; 2] = [700, 500];
const API_VERSION: ApiVersion = ApiVersion { major: 1, minor: 2 };

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

fn main() -> Result<(), Box<dyn Error>> {
    info!("starting triangle example...");

    let entry = create_entry()?;
    info!("vulkan loaded");

    let event_loop = EventLoop::new()?;
    let window_builder =
        WindowBuilder::new()
            .with_title(TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(
                DEFAULT_WINDOW_SIZE[0],
                DEFAULT_WINDOW_SIZE[1],
            ));
    let window = window_builder.build(&event_loop)?;
    let display_handle = window.display_handle()?;
    let window_handle = window.window_handle()?;
    info!("created window");

    let empty_str_vec = Vec::<&str>::new();
    let instance = Arc::new(Instance::new(
        entry.clone(),
        API_VERSION,
        TITLE,
        display_handle.as_raw(),
        empty_str_vec.clone(),
        empty_str_vec,
    )?);
    info!("created vulkan instance");

    let surface = Arc::new(Surface::new(
        &entry,
        instance.clone(),
        display_handle.as_raw(),
        window_handle.as_raw(),
    )?);
    info!("created surface");

    let physical_device_handles = unsafe { instance.inner().enumerate_physical_devices() }?;
    let physical_device_handle = physical_device_handles
        .get(0)
        .ok_or(BortExampleError::NoPhysicalDevice)?;
    let physical_device = Arc::new(PhysicalDevice::new(
        instance.clone(),
        *physical_device_handle,
    )?);
    info!("created physical device");

    let (queue_family_index, _queue_family_properties) = physical_device
        .queue_family_properties()
        .into_iter()
        .enumerate() // because we want the queue family index
        .find(|&(queue_family_index, queue_family_properties)| {
            let graphics_support = queue_family_properties
                .queue_flags
                .contains(vk::QueueFlags::GRAPHICS);
            let surface_support = surface
                .get_physical_device_surface_support(&physical_device, queue_family_index as u32)
                .unwrap_or(false);
            graphics_support && surface_support
        })
        .ok_or(BortExampleError::NoSuitableQueueFamily)?;

    let queue_priorities = [1.0];
    let queue_create_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(queue_family_index as u32)
        .queue_priorities(&queue_priorities);
    let queue_create_infos = [queue_create_info.build()];

    let device = Arc::new(Device::new(
        physical_device.clone(),
        &queue_create_infos,
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        [],
        [],
        None,
    )?);
    info!("created logical device");

    let queue = Arc::new(Queue::new(device.clone(), queue_family_index as u32, 0)?);
    info!("created queue");

    let surface_capabilities =
        surface.get_physical_device_surface_capabilities(&physical_device)?;
    let swapchain_image_count = surface_capabilities.min_image_count + 1;
    let surface_format = surface.get_physical_device_surface_formats(&physical_device)?[0];
    let composite_alpha = choose_composite_alpha(surface_capabilities);
    let swapchain_properties = SwapchainProperties::new_default(
        &device,
        &surface,
        swapchain_image_count,
        surface_format,
        composite_alpha,
        vk::ImageUsageFlags::COLOR_ATTACHMENT,
        window.inner_size().into(),
    )?;
    let swapchain = Arc::new(Swapchain::new(
        device.clone(),
        surface.clone(),
        swapchain_properties,
    )?);
    let shaders_write_linear_color = is_format_srgb(swapchain.properties().surface_format.format);
    info!("created swapchain");

    let swapchain_image_views = swapchain
        .swapchain_images()
        .iter()
        .map(|swapchain_image| {
            ImageView::new(swapchain_image.clone(), swapchain.image_view_properties())
        })
        .collect::<Result<Vec<_>, _>>()?;
    info!("created swapchain image views");

    // todo render pass

    let memory_allocator = Arc::new(MemoryAllocator::new(device.clone())?);
    info!("created memory allocator");

    let command_pool_properties = CommandPoolProperties {
        flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        queue_family_index: queue_family_index as u32,
    };
    let command_pool = Arc::new(CommandPool::new(device.clone(), command_pool_properties)?);
    info!("created command pool");

    Ok(())
}

// ~~ Errors ~~

#[derive(Debug, Clone, Copy)]
enum BortExampleError {
    NoPhysicalDevice,
    NoSuitableQueueFamily,
}

impl std::fmt::Display for BortExampleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NoPhysicalDevice => write!(f, "no vulkan physical device available"),
            Self::NoSuitableQueueFamily => write!(
                f,
                "no queue family was found that supports surface and graphics operations"
            ),
        }
    }
}

impl std::error::Error for BortExampleError {}
