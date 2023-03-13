# TODO

- properties from create_info_builder
	- comments on what defaults make sense (part of the reason properties even exist...)
	- is pnext chain accessible during creation?
- write descriptor sets store resource refs e.g. buffer/image (same for command buffers at some point...)
	- have take fn for these so they can be passed onto command buffer etc
- spec constants
- instance and device properties
- cargo feature to use rc instead of arc
- make all getters inline for consistency (don't know where the chains will be)

create_info Properties struct checklist:
- derive Clone
- impl Default (or derive in special cases) right after struct declaration
- document which members have nonsense default values
- new_default() fn with these values as args
- write_create_info_builder<'a>() fn _note: this doesn't always need `&'a self`_
- create_info_builder() fn (optional)
- From<&vk::*CreateInfoBuilder>

other:
- new_from_create_info() fn for associated struct
- // Properties comment
- inline getters