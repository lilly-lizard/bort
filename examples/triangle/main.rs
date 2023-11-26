use ash::{prelude::VkResult, vk};
use bort_vk::{
    choose_composite_alpha, is_format_srgb, ApiVersion, ColorBlendState, CommandPool,
    CommandPoolProperties, Device, DynamicState, Fence, Framebuffer, FramebufferProperties,
    GraphicsPipeline, GraphicsPipelineProperties, ImageView, ImageViewAccess, Instance,
    PhysicalDevice, PipelineLayout, PipelineLayoutProperties, Queue, RenderPass, Semaphore,
    ShaderModule, ShaderStage, Subpass, Surface, Swapchain, SwapchainProperties, ViewportState,
};
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{error::Error, ffi::CString, sync::Arc};
use winit::{event_loop::EventLoop, window::WindowBuilder};

const TITLE: &str = "Triangle (bort example)";
const DEFAULT_WINDOW_SIZE: [u32; 2] = [700, 500];
const API_VERSION: ApiVersion = ApiVersion { major: 1, minor: 2 };
const MAX_FRAMES_IN_FLIGHT: usize = 2;

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

    let device = Arc::new(Device::new(
        physical_device.clone(),
        &[queue_create_info.build()],
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        ["VK_KHR_swapchain".to_string()],
        [],
        None,
    )?);
    info!("created logical device");

    let queue = Arc::new(Queue::new(device.clone(), queue_family_index as u32, 0)?);
    info!("created queue");

    let surface_capabilities =
        surface.get_physical_device_surface_capabilities(&physical_device)?;
    let preferred_swapchain_image_count = surface_capabilities.min_image_count + 1;
    let surface_format = surface.get_physical_device_surface_formats(&physical_device)?[0];
    let composite_alpha = choose_composite_alpha(surface_capabilities);
    let swapchain_properties = SwapchainProperties::new_default(
        &device,
        &surface,
        preferred_swapchain_image_count,
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
            let image_view =
                ImageView::new(swapchain_image.clone(), swapchain.image_view_properties())?;
            Ok(Arc::new(image_view))
        })
        .collect::<VkResult<Vec<_>>>()?;
    info!(
        "created {} swapchain image views",
        swapchain_image_views.len()
    );

    let swapchain_attachment_description = vk::AttachmentDescription::builder()
        .format(surface_format.format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    let swapchain_attachemnt_reference = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let subpass = Subpass::new(&[swapchain_attachemnt_reference.build()], None, &[]);

    let image_aquire_subpass_dependency = vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

    let render_pass = Arc::new(RenderPass::new(
        device.clone(),
        [swapchain_attachment_description.build()],
        [subpass],
        [image_aquire_subpass_dependency.build()],
    )?);
    info!("created render pass");

    let pipeline_layout_properties = PipelineLayoutProperties::new(Vec::new(), Vec::new());
    let pipeline_layout = Arc::new(PipelineLayout::new(
        device.clone(),
        pipeline_layout_properties,
    )?);

    let mut vertex_spv_file = std::io::Cursor::new(&include_bytes!("./triangle.vert.spv")[..]);
    let vert_shader = Arc::new(ShaderModule::new_from_spirv(
        device.clone(),
        &mut vertex_spv_file,
    )?);
    let vert_stage = ShaderStage::new(
        vk::ShaderStageFlags::VERTEX,
        vert_shader,
        CString::new("main")?,
        None,
    );

    let mut frag_spv_file = std::io::Cursor::new(&include_bytes!("./triangle.frag.spv")[..]);
    let frag_shader = Arc::new(ShaderModule::new_from_spirv(
        device.clone(),
        &mut frag_spv_file,
    )?);
    let frag_stage = ShaderStage::new(
        vk::ShaderStageFlags::FRAGMENT,
        frag_shader,
        CString::new("main")?,
        None,
    );

    let dynamic_state =
        DynamicState::new_default(vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]);
    let viewport_state = ViewportState::new_dynamic(1, 1);
    let color_blend_state =
        ColorBlendState::new_default(vec![ColorBlendState::blend_state_disabled()]);

    let mut pipeline_properties = GraphicsPipelineProperties::default();
    pipeline_properties.subpass_index = 0;
    pipeline_properties.dynamic_state = dynamic_state;
    pipeline_properties.color_blend_state = color_blend_state;
    pipeline_properties.viewport_state = viewport_state;

    let pipeline = GraphicsPipeline::new(
        pipeline_layout,
        pipeline_properties,
        &[vert_stage, frag_stage],
        &render_pass,
        None,
    )?;
    info!("created graphics pipeline");

    let framebuffers = swapchain_image_views
        .into_iter()
        .map(|swapchain_image_view| {
            let attachments: Vec<Arc<dyn ImageViewAccess>> = vec![swapchain_image_view.clone()];

            let framebuffer_properties = FramebufferProperties::new_default(
                attachments,
                swapchain_image_view.image().dimensions(),
            );

            let framebuffer = Framebuffer::new(render_pass.clone(), framebuffer_properties)?;
            Ok(Arc::new(framebuffer))
        })
        .collect::<VkResult<Vec<_>>>()?;
    info!("created {} framebuffers", framebuffers.len());

    let command_pool_properties = CommandPoolProperties {
        flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        queue_family_index: queue_family_index as u32,
    };
    let command_pool = Arc::new(CommandPool::new(device.clone(), command_pool_properties)?);
    info!("created command pool");

    let command_buffers = command_pool
        .allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, framebuffers.len() as u32)?;
    info!("allocated command buffers");

    let mut image_available_semaphores: Vec<Semaphore> = Vec::new();
    let mut render_finished_semaphores: Vec<Semaphore> = Vec::new();
    let mut in_flight_fences: Vec<Fence> = Vec::new();
    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        image_available_semaphores.push(Semaphore::new(device.clone())?);
        render_finished_semaphores.push(Semaphore::new(device.clone())?);
        in_flight_fences.push(Fence::new_signalled(device.clone())?);
    }
    info!("created semaphores and fences");

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
