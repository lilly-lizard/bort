# TODO

- move stuff in memory.rs to vk_mem_bort and rename bort_vma
	- see pool.rs, got Arc dependency and Drop and everything!
- make all getters inline for consistency (don't know where the chains will be)
- properties from create_info_builder
- write descriptor sets
- spec constants
- instance and device properties
- cargo feature to use rc instead of arc