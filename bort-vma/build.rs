#[cfg(feature = "generate_bindings")]
extern crate bindgen;
extern crate cc;

use std::env;

fn main() {
    let mut build = cc::Build::new();

    // Disable VMA_ASSERT when rust assertions are disabled
    #[cfg(not(debug_assertions))]
    build.define("NDEBUG", "");

    // We want to use the loader in ash, instead of requiring us to link
    // in vulkan.dll/.dylib in addition to ash. This is especially important
    // for MoltenVK, where there is no default installation path, unlike
    // Linux (pkconfig) and Windows (VULKAN_SDK environment variable).
    build.define("VMA_STATIC_VULKAN_FUNCTIONS", "0");

    // This prevents VMA from trying to fetch any remaining pointers
    // that are still null after using the loader in ash, which can
    // cause linker errors.
    build.define("VMA_DYNAMIC_VULKAN_FUNCTIONS", "0");

    // feature flags

    #[cfg(feature = "vulkan-1-4")]
    build.define("VMA_VULKAN_VERSION", "1004000");
    #[cfg(feature = "vulkan-1-3")]
    build.define("VMA_VULKAN_VERSION", "1003000");
    #[cfg(feature = "vulkan-1-2")]
    build.define("VMA_VULKAN_VERSION", "1002000");
    #[cfg(feature = "vulkan-1-1")]
    build.define("VMA_VULKAN_VERSION", "1001000");
    #[cfg(feature = "vulkan-1-0")]
    build.define("VMA_VULKAN_VERSION", "1000000");

    #[cfg(feature = "recording-enabled")]
    build.define("VMA_RECORDING_ENABLED", "1");

    #[cfg(feature = "debug-always-dedicated-memory")]
    build.define("VMA_DEBUG_ALWAYS_DEDICATED_MEMORY", "1");

    #[cfg(feature = "debug-initialize-allocations")]
    build.define("VMA_DEBUG_INITIALIZE_ALLOCATIONS", "1");

    #[cfg(feature = "debug-global-mutex")]
    build.define("VMA_DEBUG_GLOBAL_MUTEX", "1");

    #[cfg(feature = "debug-dont-exceed-max-memory-allocation-count")]
    build.define("VMA_DEBUG_DONT_EXCEED_MAX_MEMORY_ALLOCATION_COUNT", "1");

    build.include("vendor/VulkanMemoryAllocator/include");
    build.include("vendor/Vulkan-Headers/include");

    // Add the files we build
    build.file("wrapper/vma_lib.cpp");

    let target = env::var("TARGET").unwrap();
    if target.contains("darwin") {
        build
            .flag("-std=c++17")
            .flag("-Wno-missing-field-initializers")
            .flag("-Wno-unused-variable")
            .flag("-Wno-unused-parameter")
            .flag("-Wno-unused-private-field")
            .flag("-Wno-reorder")
            .flag("-Wno-nullability-completeness")
            .cpp_link_stdlib("c++")
            .cpp_set_stdlib("c++")
            .cpp(true);
    } else if target.contains("ios") {
        build
            .flag("-std=c++17")
            .flag("-Wno-missing-field-initializers")
            .flag("-Wno-unused-variable")
            .flag("-Wno-unused-parameter")
            .flag("-Wno-unused-private-field")
            .flag("-Wno-reorder")
            .cpp_link_stdlib("c++")
            .cpp_set_stdlib("c++")
            .cpp(true);
    } else if target.contains("android") {
        build
            .flag("-std=c++17")
            .flag("-Wno-missing-field-initializers")
            .flag("-Wno-unused-variable")
            .flag("-Wno-unused-parameter")
            .flag("-Wno-unused-private-field")
            .flag("-Wno-reorder")
            .cpp_link_stdlib("c++")
            .cpp(true);
    } else if target.contains("linux") {
        build
            .flag("-std=c++17")
            .flag("-Wno-missing-field-initializers")
            .flag("-Wno-unused-variable")
            .flag("-Wno-unused-parameter")
            .flag("-Wno-unused-private-field")
            .flag("-Wno-reorder")
            .cpp_link_stdlib("stdc++")
            .cpp(true);
    } else if target.contains("windows") && target.contains("gnu") {
        build
            .flag("-std=c++17")
            .flag("-Wno-missing-field-initializers")
            .flag("-Wno-unused-variable")
            .flag("-Wno-unused-parameter")
            .flag("-Wno-unused-private-field")
            .flag("-Wno-reorder")
            .flag("-Wno-type-limits")
            .cpp_link_stdlib("stdc++")
            .cpp(true);
    }

    build.compile("vma");

    generate_bindings("src/ffi.rs");
}

#[cfg(feature = "generate_bindings")]
fn generate_bindings(output_file: &str) {
    let bindings = bindgen::Builder::default()
        .clang_arg("-I./wrapper")
        .clang_arg("-I./vendor/Vulkan-Headers/include")
        .header("vendor/VulkanMemoryAllocator/include/vk_mem_alloc.h")
        .rustfmt_bindings(true)
        .size_t_is_usize(true)
        .blocklist_type("__darwin_.*")
        .allowlist_function("vma.*")
        .allowlist_function("PFN_vma.*")
        .allowlist_type("Vma.*")
        .parse_callbacks(Box::new(FixAshTypes))
        .blocklist_type("Vk.*")
        .blocklist_type("PFN_vk.*")
        .raw_line("#![allow(non_camel_case_types)]")
        .raw_line("#![allow(non_snake_case)]")
        .raw_line("#![allow(dead_code)]")
        .raw_line("use ash::vk::*;")
        .trust_clang_mangling(false)
        .layout_tests(false)
        .rustified_enum("Vma.*")
        .generate()
        .expect("Unable to generate bindings!");

    bindings
        .write_to_file(std::path::Path::new(output_file))
        .expect("Unable to write bindings!");
}

#[cfg(not(feature = "generate_bindings"))]
fn generate_bindings(_: &str) {}

#[cfg(feature = "generate_bindings")]
#[derive(Debug)]
struct FixAshTypes;

#[cfg(feature = "generate_bindings")]
impl bindgen::callbacks::ParseCallbacks for FixAshTypes {
    fn item_name(&self, original_item_name: &str) -> Option<String> {
        if original_item_name.starts_with("Vk") {
            // Strip `Vk` prefix, will use `ash::vk::*` instead
            Some(original_item_name.trim_start_matches("Vk").to_string())
        } else if original_item_name.starts_with("PFN_vk") && original_item_name.ends_with("KHR") {
            // VMA uses a few extensions like `PFN_vkGetBufferMemoryRequirements2KHR`,
            // ash keeps these as `PFN_vkGetBufferMemoryRequirements2`
            Some(original_item_name.trim_end_matches("KHR").to_string())
        } else {
            None
        }
    }

    // When ignoring `Vk` types, bindgen loses derives for some type. Quick workaround.
    fn add_derives(&self, name: &str) -> Vec<String> {
        if name.starts_with("VmaAllocationInfo") || name.starts_with("VmaDefragmentationStats") {
            vec!["Debug".into(), "Copy".into(), "Clone".into()]
        } else {
            vec![]
        }
    }
}
