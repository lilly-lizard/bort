use ash::vk;
use std::{
    ffi::{CStr, CString, NulError},
    os::raw::c_char,
    str::Utf8Error,
};

pub fn string_to_c_string_vec(
    source: impl IntoIterator<Item = String>,
) -> Result<Vec<*const c_char>, NulError> {
    source
        .into_iter()
        .map(|name| Ok(CString::new(name)?.as_ptr()))
        .collect()
}

/// Safety: see [`CStr::from_ptr`](std::ffi::CStr::from_ptr) documentation...
pub unsafe fn c_string_to_string(source: *const c_char) -> Result<String, Utf8Error> {
    let c_str = CStr::from_ptr(source);
    Ok(c_str.to_str()?.to_string())
}

pub fn is_format_srgb(format: vk::Format) -> bool {
    let format_string = format!("{:?}", format);
    format_string.contains("SRGB")
}
