//! Uses code from `ash-window` for surface creation from raw window handle.
//! Original source found (here)[https://github.com/ash-rs/ash/blob/master/ash-window/src/lib.rs]

use crate::{is_format_linear, is_format_srgb, Instance, PhysicalDevice, ALLOCATION_CALLBACK_NONE};
#[cfg(feature = "raw-window-handle-06")]
use ash::vk::{HINSTANCE, HWND};
use ash::{extensions::khr, prelude::VkResult, vk, Entry};
#[cfg(feature = "raw-window-handle-05")]
use raw_window_handle_05::{RawDisplayHandle, RawWindowHandle};
#[cfg(feature = "raw-window-handle-06")]
use raw_window_handle_06::{RawDisplayHandle, RawWindowHandle};
use std::{error, fmt, sync::Arc};

pub struct Surface {
    handle: vk::SurfaceKHR,
    surface_loader: khr::Surface,

    // dependencies
    instance: Arc<Instance>,
}

impl Surface {
    pub fn new(
        entry: &Entry,
        instance: Arc<Instance>,
        raw_display_handle: RawDisplayHandle,
        raw_window_handle: RawWindowHandle,
    ) -> Result<Self, SurfaceCreationError> {
        let handle = unsafe {
            create_vk_surface(
                entry,
                instance.inner(),
                raw_display_handle,
                raw_window_handle,
                ALLOCATION_CALLBACK_NONE,
            )
        }?;

        let surface_loader = khr::Surface::new(&entry, instance.inner());

        Ok(Self {
            handle,
            surface_loader,

            instance,
        })
    }

    pub fn get_physical_device_surface_support(
        &self,
        physical_device: &PhysicalDevice,
        queue_family_index: u32,
    ) -> VkResult<bool> {
        unsafe {
            self.surface_loader.get_physical_device_surface_support(
                physical_device.handle(),
                queue_family_index,
                self.handle,
            )
        }
    }

    pub fn get_physical_device_surface_capabilities(
        &self,
        physical_device: &PhysicalDevice,
    ) -> VkResult<vk::SurfaceCapabilitiesKHR> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device.handle(), self.handle)
        }
    }

    pub fn get_physical_device_surface_formats(
        &self,
        physical_device: &PhysicalDevice,
    ) -> VkResult<Vec<vk::SurfaceFormatKHR>> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(physical_device.handle(), self.handle)
        }
    }

    pub fn get_physical_device_surface_present_modes(
        &self,
        physical_device: &PhysicalDevice,
    ) -> VkResult<Vec<vk::PresentModeKHR>> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_present_modes(physical_device.handle(), self.handle)
        }
    }

    // Getters

    pub fn handle(&self) -> vk::SurfaceKHR {
        self.handle
    }

    pub fn surface_loader(&self) -> &khr::Surface {
        &self.surface_loader
    }

    pub fn instance(&self) -> &Arc<Instance> {
        &self.instance
    }
}

/// Create a surface from a raw surface handle.
///
/// _Note: this function was copied from [ash](https://github.com/ash-rs/ash) to allow for better
/// dependency control._
///
/// `instance` must have created with platform specific surface extensions enabled, acquired
/// through [`enumerate_required_extensions()`].
///
/// # Safety
///
/// There is a [parent/child relation] between [`Instance`] and [`Entry`], and the resulting
/// [`vk::SurfaceKHR`].  The application must not [destroy][Instance::destroy_instance()] these
/// parent objects before first [destroying][khr::Surface::destroy_surface()] the returned
/// [`vk::SurfaceKHR`] child object.  [`vk::SurfaceKHR`] does _not_ implement [drop][drop()]
/// semantics and can only be destroyed via [`destroy_surface()`][khr::Surface::destroy_surface()].
///
/// See the [`Entry::create_instance()`] documentation for more destruction ordering rules on
/// [`Instance`].
///
/// The window represented by `window_handle` must be associated with the display connection
/// in `display_handle`.
///
/// `window_handle` and `display_handle` must be associated with a valid window and display
/// connection, which must not be destroyed for the lifetime of the returned [`vk::SurfaceKHR`].
///
/// [parent/child relation]: https://registry.khronos.org/vulkan/specs/1.3-extensions/html/vkspec.html#fundamentals-objectmodel-lifetime
unsafe fn create_vk_surface(
    entry: &Entry,
    instance: &ash::Instance,
    display_handle: RawDisplayHandle,
    window_handle: RawWindowHandle,
    allocation_callbacks: Option<&vk::AllocationCallbacks>,
) -> Result<vk::SurfaceKHR, SurfaceCreationError> {
    match (display_handle, window_handle) {
        (RawDisplayHandle::Windows(_), RawWindowHandle::Win32(window)) => {
            #[cfg(feature = "raw-window-handle-05")]
            let hinstance = window.hinstance;
            #[cfg(feature = "raw-window-handle-06")]
            let hinstance = window
                .hinstance
                .ok_or(SurfaceCreationError::NoWin32HINSTANCE)?
                .get() as HINSTANCE;

            #[cfg(feature = "raw-window-handle-05")]
            let hwnd = window.hwnd;
            #[cfg(feature = "raw-window-handle-06")]
            let hwnd = window.hwnd.get() as HWND;

            let surface_desc = vk::Win32SurfaceCreateInfoKHR::builder()
                .hinstance(hinstance)
                .hwnd(hwnd);
            let surface_fn = khr::Win32Surface::new(entry, instance);
            let surface_handle =
                surface_fn.create_win32_surface(&surface_desc, allocation_callbacks)?;
            Ok(surface_handle)
        }

        (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) => {
            #[cfg(feature = "raw-window-handle-05")]
            let display_wl = display.display;
            #[cfg(feature = "raw-window-handle-06")]
            let display_wl = display.display.as_ptr();

            #[cfg(feature = "raw-window-handle-05")]
            let surface_wl = window.surface;
            #[cfg(feature = "raw-window-handle-06")]
            let surface_wl = window.surface.as_ptr();

            let surface_desc = vk::WaylandSurfaceCreateInfoKHR::builder()
                .display(display_wl)
                .surface(surface_wl);
            let surface_fn = khr::WaylandSurface::new(entry, instance);
            let surface_handle =
                surface_fn.create_wayland_surface(&surface_desc, allocation_callbacks)?;
            Ok(surface_handle)
        }

        (RawDisplayHandle::Xlib(display), RawWindowHandle::Xlib(window)) => {
            #[cfg(feature = "raw-window-handle-05")]
            let display_x = display.display;
            #[cfg(feature = "raw-window-handle-06")]
            let display_x = display
                .display
                .ok_or(SurfaceCreationError::NoXlibDisplayPointer)?
                .as_ptr();

            let surface_desc = vk::XlibSurfaceCreateInfoKHR::builder()
                .dpy(display_x.cast())
                .window(window.window);
            let surface_fn = khr::XlibSurface::new(entry, instance);
            let surface_handle =
                surface_fn.create_xlib_surface(&surface_desc, allocation_callbacks)?;
            Ok(surface_handle)
        }

        (RawDisplayHandle::Xcb(display), RawWindowHandle::Xcb(window)) => {
            #[cfg(feature = "raw-window-handle-05")]
            let connection_xcb = display.connection;
            #[cfg(feature = "raw-window-handle-06")]
            let connection_xcb = display
                .connection
                .ok_or(SurfaceCreationError::NoXcbConnectionPointer)?
                .as_ptr();

            #[cfg(feature = "raw-window-handle-05")]
            let window_xcb = window.window;
            #[cfg(feature = "raw-window-handle-06")]
            let window_xcb = window.window.get();

            let surface_desc = vk::XcbSurfaceCreateInfoKHR::builder()
                .connection(connection_xcb)
                .window(window_xcb);
            let surface_fn = khr::XcbSurface::new(entry, instance);
            let surface_handle =
                surface_fn.create_xcb_surface(&surface_desc, allocation_callbacks)?;
            Ok(surface_handle)
        }

        (RawDisplayHandle::Android(_), RawWindowHandle::AndroidNdk(window)) => {
            #[cfg(feature = "raw-window-handle-05")]
            let window_android = window.a_native_window;
            #[cfg(feature = "raw-window-handle-06")]
            let window_android = window.a_native_window.as_ptr();

            let surface_desc = vk::AndroidSurfaceCreateInfoKHR::builder().window(window_android);
            let surface_fn = khr::AndroidSurface::new(entry, instance);
            let surface_handle =
                surface_fn.create_android_surface(&surface_desc, allocation_callbacks)?;
            Ok(surface_handle)
        }

        #[cfg(target_os = "macos")]
        (RawDisplayHandle::AppKit(_), RawWindowHandle::AppKit(window)) => {
            use ash::extensions::ext::MetalSurface;
            #[cfg(feature = "raw-window-handle-05")]
            use raw_window_metal_03::{appkit, Layer};
            #[cfg(feature = "raw-window-handle-06")]
            use raw_window_metal_04::{appkit, Layer};

            #[cfg(feature = "raw-window-handle-05")]
            let layer = match appkit::metal_layer_from_handle(window) {
                Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
                Layer::None => return Err(vk::Result::ERROR_INITIALIZATION_FAILED.into()),
            };
            #[cfg(feature = "raw-window-handle-06")]
            let layer = match appkit::metal_layer_from_handle(window) {
                Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
            };

            let surface_desc = vk::MetalSurfaceCreateInfoEXT::builder().layer(&*layer);
            let surface_fn = MetalSurface::new(entry, instance);
            let surface_handle =
                surface_fn.create_metal_surface(&surface_desc, allocation_callbacks)?;
            Ok(surface_handle)
        }

        #[cfg(target_os = "ios")]
        (RawDisplayHandle::UiKit(_), RawWindowHandle::UiKit(window)) => {
            #[cfg(feature = "raw-window-handle-05")]
            use raw_window_metal_03::{uikit, Layer};
            #[cfg(feature = "raw-window-handle-06")]
            use raw_window_metal_04::{uikit, Layer};

            #[cfg(feature = "raw-window-handle-05")]
            let layer = match uikit::metal_layer_from_handle(window) {
                Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
                Layer::None => return Err(vk::Result::ERROR_INITIALIZATION_FAILED.into()),
            };
            #[cfg(feature = "raw-window-handle-06")]
            let layer = match uikit::metal_layer_from_handle(window) {
                Layer::Existing(layer) | Layer::Allocated(layer) => layer.cast(),
            };

            let surface_desc = vk::MetalSurfaceCreateInfoEXT::builder().layer(&*layer);
            let surface_fn = ext::MetalSurface::new(entry, instance);
            let surface_handle =
                surface_fn.create_metal_surface(&surface_desc, allocation_callbacks)?;
            Ok(surface_handle)
        }

        _ => Err(SurfaceCreationError::UnsupportedDisplaySystem()),
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader
                .destroy_surface(self.handle, ALLOCATION_CALLBACK_NONE)
        };
    }
}

/// Returns the first surface format with a linear image format in the vec. Returns `None` is there's none.
pub fn get_first_srgb_surface_format(
    surface_formats: &Vec<vk::SurfaceFormatKHR>,
) -> Option<vk::SurfaceFormatKHR> {
    surface_formats
        .iter()
        .cloned()
        // use the first SRGB format we find
        .find(|vk::SurfaceFormatKHR { format, .. }| is_format_srgb(*format))
}

/// Returns the first surface format with a linear image format in the vec. Returns `None` is there's none.
pub fn get_first_linear_surface_format(
    surface_formats: &Vec<vk::SurfaceFormatKHR>,
) -> Option<vk::SurfaceFormatKHR> {
    surface_formats
        .iter()
        .cloned()
        // use the first linear format we find
        .find(|vk::SurfaceFormatKHR { format, .. }| is_format_linear(*format))
}

// ~~ Errors ~~

#[derive(Clone, Copy, Debug)]
pub enum SurfaceCreationError {
    VkResult(vk::Result),
    NoXcbConnectionPointer,
    NoXlibDisplayPointer,
    NoWin32HINSTANCE,
    UnsupportedDisplaySystem(),
}

impl fmt::Display for SurfaceCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VkResult(e) => write!(f, "{}", e),
            Self::NoXcbConnectionPointer => write!(
                f,
                "the XCB display handle is missing a valid xcb_connection_t* pointer which is required
                to create a surface"
            ),
            Self::NoXlibDisplayPointer => write!(
                f,
                "the X11 display handle is missing a valid Display* pointer which is required
                to create a surface"
            ),
            Self::NoWin32HINSTANCE => write!(
                f,
                "the Win32 window handle is missing a valid HINSTANCE which is required
                to create a surface"
            ),
            Self::UnsupportedDisplaySystem() => write!(
                f,
                "the display and window handles represent a windowing system that is currently unsupported."
            )
        }
    }
}

impl error::Error for SurfaceCreationError {}

impl From<vk::Result> for SurfaceCreationError {
    fn from(res: vk::Result) -> Self {
        Self::VkResult(res)
    }
}
