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