#[cfg(feature = "raw-window-handle-05")]
pub use raw_window_handle_05 as raw_window_handle;
#[cfg(feature = "raw-window-handle-06")]
pub use raw_window_handle_06 as raw_window_handle;

mod buffer;
mod command_buffer;
mod command_pool;
mod common;
mod debug_callback;
mod descriptor_layout;
mod descriptor_pool;
mod descriptor_set;
mod device;
mod fence;
mod framebuffer;
mod image;
mod image_view;
mod instance;
mod memory_allocation;
mod memory_allocator;
mod memory_pool;
mod physical_device;
mod pipeline_access;
mod pipeline_cache;
mod pipeline_compute;
mod pipeline_graphics;
mod pipeline_layout;
mod queue;
mod render_pass;
mod sampler;
mod semaphore;
mod shader_module;
mod surface;
mod swapchain;

// so you can access everything from the `bort_vma` namespace instead of typing something like
// `bort_vma::pipeline_compute::ComputePipeline`
pub use buffer::*;
pub use command_buffer::*;
pub use command_pool::*;
pub use common::*;
pub use debug_callback::*;
pub use descriptor_layout::*;
pub use descriptor_pool::*;
pub use descriptor_set::*;
pub use device::*;
pub use fence::*;
pub use framebuffer::*;
pub use image::*;
pub use image_view::*;
pub use instance::*;
pub use memory_allocation::*;
pub use memory_allocator::*;
pub use memory_pool::*;
pub use physical_device::*;
pub use pipeline_access::*;
pub use pipeline_cache::*;
pub use pipeline_compute::*;
pub use pipeline_graphics::*;
pub use pipeline_layout::*;
pub use queue::*;
pub use render_pass::*;
pub use sampler::*;
pub use semaphore::*;
pub use shader_module::*;
pub use surface::*;
pub use swapchain::*;
