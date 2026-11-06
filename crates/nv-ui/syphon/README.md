Syphon's Metal shaders, from the [Syphon Framework](https://github.com/Syphon/Syphon-Framework)
(BSD licence, see `NOTICE.md`), unchanged.

Linked statically into NeanderthalVJ, Syphon looks for its compiled shaders,
`default.metallib`, beside the program rather than in a framework bundle.
`build.rs` compiles them there when the `syphon` feature is on.
