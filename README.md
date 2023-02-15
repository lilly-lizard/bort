# Bort

Is a pretty unambitious, lightweight vulkan wrapper on top of [ash](https://github.com/ash-rs/ash) aiming to reduce some boilerplate, call destructors with `Drop`, reference count resource dependencies, store create-info properties etc.

![Bort under attack](/assets/bort-under-attack.jpg)

Oh, also this is like all unsafe from the vulkan spec perspective i.e. there's very little spec validity checking. I don't really care because that's what the validation layers are for so I cbf marking everything as `unsafe` for now.