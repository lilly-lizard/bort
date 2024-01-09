use ash::vk;
use std::{ffi::CStr, os::raw::c_char, str::Utf8Error};

/// # Safety
/// See [`CStr::from_ptr`](std::ffi::CStr::from_ptr) documentation...
pub unsafe fn c_string_to_string(source: *const c_char) -> Result<String, Utf8Error> {
    let c_str = CStr::from_ptr(source);
    Ok(c_str.to_str()?.to_string())
}

pub fn is_format_srgb(format: vk::Format) -> bool {
    let format_string = format!("{:?}", format);
    format_string.contains("SRGB")
}

pub fn is_format_linear(format: vk::Format) -> bool {
    !is_format_srgb(format)
}
