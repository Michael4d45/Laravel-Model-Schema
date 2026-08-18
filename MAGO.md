# Mago crate notes (in-process, not the CLI)

Version pin: `mago-syntax` / `mago-names` / `mago-database` / `mago-allocator` **1.46**. `mago-docblock` **1.42**.

Parse with `parse_file` into an arena-allocated `Program`. Walk it and copy what you need into owned Rust structs. Do not store `&Program` past the arena.

Do not treat mago CLI JSON as the parser.
