# TODO

- blend_state_alpha() has something to do with surface blend mode???
- only create_info write vecs when len != 0 (see descriptor_layout.rs immutable samplers)
- replace all `impl Iterator` with slices (will cause probs later with open source contributers)
- properties from create_info_builder
	- comments on what defaults make sense (part of the reason properties even exist...)
	- is pnext chain accessible during creation?
- write descriptor sets store resource refs e.g. buffer/image (same for command buffers at some point...)
	- have take fn for these so they can be passed onto command buffer etc
- spec constants
- instance and device properties
- cargo feature to use rc instead of arc
- make all getters inline for consistency (don't know where the chains will be)

# create_info Properties struct checklist

todo remove all From impls and have explicit fns.
todo check naming of `from_create_info` and `from_create_info_builder`. be explicit becuase they're different!
review command_pool and buffer again...

- derive `Clone`
- `impl Default` (or derive in special cases. check all derives at end!) right after struct declaration
- document which members have nonsense default values
- `new_default()` fn with these values as args
- `write_create_info_builder<'a>()` fn _note: this doesn't always need `&'a self`_
- `create_info_builder()` fn (optional)
- `From<&vk::*CreateInfoBuilder>`

other:
- `new_from_create_info_builder()` fn for associated struct
- // Properties comment
- inline getters
