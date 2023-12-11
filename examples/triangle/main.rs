use ash::{prelude::VkResult, vk};
use bort_vk::{
    choose_composite_alpha, is_format_srgb, ApiVersion, ColorBlendState, CommandBuffer,
    CommandPool, CommandPoolProperties, DebugCallback, DebugCallbackProperties, Device,
    DeviceOwned, DynamicState, Fence, Framebuffer, FramebufferProperties, GraphicsPipeline,
    GraphicsPipelineProperties, ImageView, ImageViewAccess, Instance, PhysicalDevice,
    PipelineLayout, PipelineLayoutProperties, Queue, RenderPass, Semaphore, ShaderModule,
    ShaderStage, Subpass, Surface, Swapchain, SwapchainImage, SwapchainProperties, ViewportState,
};
use env_logger::Env;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{
    borrow::Cow,
    error::Error,
    ffi::{CStr, CString},
    sync::Arc,
};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

const TITLE: &str = "Triangle (bort example)";
const DEFAULT_WINDOW_SIZE: [u32; 2] = [700, 500];
const API_VERSION: ApiVersion = ApiVersion { major: 1, minor: 0 };
const MAX_FRAMES_IN_FLIGHT: usize = 2;
const FENCE_TIMEOUT: u64 = 1_000_000_000;
const ENABLE_VULKAN_VALIDATION: bool = cfg!(debug_assertions);
const VALIDATION_LAYER_NAME: &str = "VK_LAYER_KHRONOS_validation";
const DEBUG_UTILS_EXTENSION_NAME: &str = "VK_EXT_debug_utils";

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
    let log_env = Env::default()
        .filter_or("MY_LOG_LEVEL", "debug")
        .write_style_or("MY_LOG_STYLE", "always");
    env_logger::init_from_env(log_env);
    info!("starting triangle example...");

    let event_loop = EventLoop::new()?;
    let window_builder =
        WindowBuilder::new()
            .with_title(TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(
                DEFAULT_WINDOW_SIZE[0],
                DEFAULT_WINDOW_SIZE[1],
            ));
    let window = window_builder.build(&event_loop)?;
    info!("created window");

    let mut engine = TriangleExample::new(window)?;

    event_loop.run(move |event, elwt| {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            state: ElementState::Released,
                            ..
                        },
                    ..
                } => elwt.exit(),
                _ => engine.draw_frame().unwrap(),
            }
        }
    })?;

    Ok(())
}

struct TriangleExample {
    window: Arc<Window>,
    surface: Arc<Surface>,
    swapchain: Arc<Swapchain>,

    queue: Arc<Queue>,
    pipeline: GraphicsPipeline,
    render_pass: Arc<RenderPass>,

    // per swapchain image
    framebuffers: Vec<Arc<Framebuffer>>,
    command_buffers: Vec<CommandBuffer>,

    // per frame in flight
    image_available_semaphores: Vec<Semaphore>,
    render_finished_semaphores: Vec<Semaphore>,
    in_flight_fences: Vec<Fence>,
    current_frame: usize,
}

impl TriangleExample {
    pub fn new(window: Window) -> Result<Self, Box<dyn Error>> {
        let display_handle = window.display_handle()?;
        let window_handle = window.window_handle()?;

        let entry = create_entry()?;
        info!("vulkan loaded");

        let mut enable_validation = ENABLE_VULKAN_VALIDATION;
        let mut instance_layers = Vec::<&str>::new();
        let mut instance_extensions = Vec::<&str>::new();

        if enable_validation {
            let layer_properties = entry.enumerate_instance_layer_properties()?;
            let extension_properties = entry.enumerate_instance_extension_properties(None)?;

            let validation_layer_installed = layer_properties.iter().any(|layer_prop| {
                let layer_name = unsafe { CStr::from_ptr(layer_prop.layer_name.as_ptr()) }
                    .to_str()
                    .unwrap();

                layer_name == VALIDATION_LAYER_NAME
            });

            let debug_utils_supported = extension_properties.iter().any(|extension_prop| {
                let extension_name =
                    unsafe { CStr::from_ptr(extension_prop.extension_name.as_ptr()) }
                        .to_str()
                        .unwrap();

                extension_name == DEBUG_UTILS_EXTENSION_NAME
            });

            if validation_layer_installed && debug_utils_supported {
                instance_layers.push(VALIDATION_LAYER_NAME);
                instance_extensions.push(DEBUG_UTILS_EXTENSION_NAME);
            } else {
                enable_validation = false;
            }
        }

        let instance = Arc::new(Instance::new(
            entry.clone(),
            API_VERSION,
            display_handle.as_raw(),
            instance_layers,
            instance_extensions,
        )?);
        info!("created vulkan instance");

        let debug_callback = if enable_validation {
            let debug_callback_properties = DebugCallbackProperties::default();
            let debug_callback = DebugCallback::new(
                instance.clone(),
                Some(log_vulkan_debug_callback),
                debug_callback_properties,
            )?;

            Some(Arc::new(debug_callback))
        } else {
            info!("vulkan validation layers disabled");
            None
        };

        let surface = Arc::new(Surface::new(
            &entry,
            instance.clone(),
            display_handle.as_raw(),
            window_handle.as_raw(),
        )?);
        info!("created surface");

        let physical_device_handles = instance.enumerate_physical_devices()?;
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
                    .get_physical_device_surface_support(
                        &physical_device,
                        queue_family_index as u32,
                    )
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
            debug_callback,
        )?);
        info!("created logical device");

        let queue = Arc::new(Queue::new(device.clone(), queue_family_index as u32, 0)?);
        info!("created queue");

        let swapchain_properties = swapchain_properties(&surface, &device, &window)?;
        let surface_format = swapchain_properties.surface_format;
        let swapchain = Arc::new(Swapchain::new(
            device.clone(),
            surface.clone(),
            swapchain_properties,
        )?);
        let _shaders_should_write_linear_color =
            is_format_srgb(swapchain.properties().surface_format.format);
        info!("created swapchain");

        let swapchain_image_views = create_swapchain_image_views(&swapchain)?;

        let render_pass = create_render_pass(device.clone(), surface_format)?;

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

        let framebuffers = create_framebuffers(swapchain_image_views, render_pass.clone())?;

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

        let current_frame = 0_usize;

        Ok(Self {
            window: Arc::new(window),
            surface,
            swapchain,

            queue,
            pipeline,
            render_pass,

            framebuffers,
            command_buffers,

            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            current_frame,
        })
    }

    pub fn draw_frame(&mut self) -> Result<(), Box<dyn Error>> {
        self.in_flight_fences[self.current_frame].wait(FENCE_TIMEOUT)?;

        let aquire_res = self.swapchain.aquire_next_image(
            FENCE_TIMEOUT,
            Some(&self.image_available_semaphores[self.current_frame]),
            None,
        );

        let (swapchain_image_index, _is_suboptimal) = match aquire_res {
            Ok(aquire_ret) => aquire_ret,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain()?;
                return Ok(());
            }
            Err(e) => return Err(e)?,
        };

        self.in_flight_fences[self.current_frame].reset()?;

        self.command_buffers[self.current_frame].reset(vk::CommandBufferResetFlags::empty())?;
        self.record_commands(
            &self.command_buffers[self.current_frame],
            swapchain_image_index as usize,
        )?;

        let wait_semaphores = [self.image_available_semaphores[self.current_frame].handle()];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[self.current_frame].handle()];
        let submit_command_buffers = [self.command_buffers[self.current_frame].handle()];

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .signal_semaphores(&signal_semaphores)
            .command_buffers(&submit_command_buffers);

        self.queue.submit(
            [submit_info],
            Some(&self.in_flight_fences[self.current_frame]),
        )?;

        let present_swapchains = [self.swapchain.handle()];
        let present_indices = [swapchain_image_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&present_swapchains)
            .image_indices(&present_indices);

        let present_res = self.swapchain.queue_present(&self.queue, &present_info);

        match present_res {
            Ok(false) => (),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) | Err(vk::Result::SUBOPTIMAL_KHR) | Ok(true) => {
                self.recreate_swapchain()?
            }
            Err(e) => return Err(e)?,
        };

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    fn record_commands(
        &self,
        command_buffer: &CommandBuffer,
        swapchain_image_index: usize,
    ) -> VkResult<()> {
        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder();
        command_buffer.begin(&command_buffer_begin_info)?;

        let clear_values = [vk::ClearValue::default()];
        let render_extent = self.framebuffers[swapchain_image_index].whole_rect();
        let viewport = self.framebuffers[self.current_frame].whole_viewport();

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass.handle())
            .framebuffer(self.framebuffers[swapchain_image_index].handle())
            .render_area(render_extent)
            .clear_values(&clear_values);
        command_buffer.begin_render_pass(&render_pass_begin_info, vk::SubpassContents::INLINE);

        command_buffer.bind_pipeline(&self.pipeline);

        command_buffer.set_viewports(&[viewport], 0);
        command_buffer.set_scissors(&[render_extent], 0);

        command_buffer.draw(3, 1, 0, 0);

        command_buffer.end_render_pass();

        command_buffer.end()?;

        Ok(())
    }

    pub fn recreate_swapchain(&mut self) -> Result<(), Box<dyn Error>> {
        info!("recreating swapchain...");

        self.queue.device().wait_idle()?;
        self.framebuffers.clear();

        let swapchain_properties =
            swapchain_properties(&self.surface, &self.queue.device(), &self.window)?;
        let surface_format = swapchain_properties.surface_format;

        self.swapchain = self.swapchain.recreate_replace(swapchain_properties)?;
        let swapchain_image_views = create_swapchain_image_views(&self.swapchain)?;

        self.render_pass = create_render_pass(self.render_pass.device().clone(), surface_format)?;
        self.framebuffers = create_framebuffers(swapchain_image_views, self.render_pass.clone())?;

        Ok(())
    }
}

impl Drop for TriangleExample {
    fn drop(&mut self) {
        info!("dropping main class...");

        let wait_res = self.queue.device().wait_idle();
        if let Err(e) = wait_res {
            error!("{}", e);
            return;
        }
    }
}

fn swapchain_properties(
    surface: &Surface,
    device: &Device,
    window: &Window,
) -> Result<SwapchainProperties, Box<dyn Error>> {
    let surface_capabilities =
        surface.get_physical_device_surface_capabilities(&device.physical_device())?;

    let preferred_swapchain_image_count = surface_capabilities.min_image_count + 1;
    let surface_format = surface.get_physical_device_surface_formats(&device.physical_device())?[0];
    let composite_alpha = choose_composite_alpha(surface_capabilities);

    let swapchain_properties = SwapchainProperties::new_default(
        device,
        surface,
        preferred_swapchain_image_count,
        surface_format,
        composite_alpha,
        vk::ImageUsageFlags::COLOR_ATTACHMENT,
        window.inner_size().into(),
    )?;

    Ok(swapchain_properties)
}

fn create_render_pass(
    device: Arc<Device>,
    surface_format: vk::SurfaceFormatKHR,
) -> Result<Arc<RenderPass>, Box<dyn Error>> {
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
        device,
        [swapchain_attachment_description.build()],
        [subpass],
        [image_aquire_subpass_dependency.build()],
    )?);

    info!("created render pass");
    Ok(render_pass)
}

fn create_swapchain_image_views(
    swapchain: &Arc<Swapchain>,
) -> Result<Vec<Arc<ImageView<SwapchainImage>>>, Box<dyn Error>> {
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

    Ok(swapchain_image_views)
}

fn create_framebuffers(
    swapchain_image_views: Vec<Arc<ImageView<SwapchainImage>>>,
    render_pass: Arc<RenderPass>,
) -> Result<Vec<Arc<Framebuffer>>, Box<dyn Error>> {
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
    Ok(framebuffers)
}

pub unsafe extern "system" fn log_vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!("Vulkan [{:?}]:\n{}", message_type, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("Vulkan [{:?}]: {}", message_type, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!("Vulkan [{:?}]: {}", message_type, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            trace!("Vulkan [{:?}]: {}", message_type, message);
        }
        _ => trace!(
            "Vulkan [{:?}] (UNKONWN SEVERITY): {}",
            message_type,
            message
        ),
    }

    vk::FALSE
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
