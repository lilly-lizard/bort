## Running the examples:

From the root of this repo...

```sh
cargo run --bin <example>
```

E.g.
```sh
cargo run --bin triangle
```

If you want to compare performances with other libraries, you should pass the `--release` flag as
well. Rust is pretty slow in debug mode.