# Bort

Bort is an unambitious, lightweight vulkan wrapper on top of [ash](https://github.com/ash-rs/ash) and [vulkan memory allocator](https://github.com/GPUOpen-LibrariesAndSDKs/VulkanMemoryAllocator/) aiming to reduce some boilerplate, call destructors with `Drop`, reference count resource dependencies (with `Arc`), store create-info properties, provide convenient defaults for create-info properties etc.

This repo consists of 2 crates:
- **bort-vk** - vulkan wrapper. [crates.io link](https://crates.io/crates/bort-vk)
- **bort-vma** - vulkan memory allocator wrapper. [crates.io link](https://crates.io/crates/bort-vma)

![Bort under attack](/assets/bort-under-attack.jpg)

Oh, also this is like all unsafe from the vulkan spec perspective i.e. there's very little spec validity checking. I don't really care because that's what the validation layers are for imo. Shout out to [vulkano](https://github.com/vulkano-rs/vulkano) for being awesome. If you want enforced spec compliance, that's the place to go!
