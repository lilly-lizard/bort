# Bort

Bort is a vulkan wrapper on top of [ash](https://github.com/ash-rs/ash) and [vulkan memory allocator](https://github.com/GPUOpen-LibrariesAndSDKs/VulkanMemoryAllocator/) aiming to...
- reduce some boilerplate
- reference count resource dependencies (with `Arc` or `Rc` depending on the `rc` feature flag)
- call destructors with `Drop` so you don't need to manage `destroy_x` functions
- store create-info properties inside objects
- provide convenient defaults for create-info properties
- while also being compatible with "raw" `ash::vk` structs

This repo consists of 2 crates:
- **bort-vk** - vulkan wrapper. [crates.io link](https://crates.io/crates/bort-vk)
- **bort-vma** - vulkan memory allocator wrapper. [crates.io link](https://crates.io/crates/bort-vma)

Example: creating a descriptor set...
```rust
{
	let layout_binding = DescriptorSetLayoutBinding {
		binding: 0,
		descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
		descriptor_count: 1,
		stage_flags: vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX,
		..Default::default()
	};
	let layout_properties = DescriptorSetLayoutProperties::new_default(vec![layout_binding]);
	// new_from_set_layout is a convenience function that automatically creates a pool and layout
	// the pool and layout are stored inside the `descriptor_set` struct
	let descriptor_set = DescriptorSet::new_from_set_layout(device, camera_layout_properties)?;

	info!("descriptor pool handle: {:?}", descriptor_set.pool().handle());
	info!("descriptor layout create flags: {:?}" descriptor_set.layout().properties().flags);
}
// free_descriptor_sets is called upon drop
```

![Bort under attack](/assets/bort-under-attack.jpg)

Oh, also there's very little vulkan spec validity checking. I don't really care because that's what the validation layers are for imo. Shout out to [vulkano](https://github.com/vulkano-rs/vulkano), if you want enforced spec compliance, that's the place to go!
