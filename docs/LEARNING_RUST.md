# Learning Rust with AkarBox

The code is commented with beginner-friendly notes. This page tells you where to find each concept, roughly in the order worth learning them.

**Start here:** read [The Rust Book](https://doc.rust-lang.org/book/) chapters 1–10 alongside this list. [Rustlings](https://github.com/rust-lang/rustlings) is great for practice.

| Concept | Where to see it | Book chapter |
|---|---|---|
| Structs, enums, `match` | `Format`, `Category`, `normalize_ext` in `crates/core/src/format.rs` | 5, 6 |
| `Option` and the `?` operator | `detect_format` in `format.rs` | 6, 9 |
| `Result` and custom errors | `crates/core/src/error.rs` (uses `thiserror`) | 9 |
| Modules and `pub use` | `crates/core/src/lib.rs` | 7 |
| Unit tests | the `#[cfg(test)]` blocks at the bottom of `format.rs` and `registry.rs` | 11 |
| Traits (interfaces) | `Converter` in `crates/core/src/converter.rs` | 10 |
| Trait objects (`Box<dyn Trait>`) | `Registry::converters` in `registry.rs` | 17 |
| Iterators and closures | `all_pairs` in `converter.rs`, `Registry::targets` | 13 |
| Collections (`HashMap`, `BinaryHeap`) | Dijkstra's algorithm in `Registry::targets` | 8 |
| Lifetimes (`'a`, `'static`) | `Step<'a>` in `converter.rs`, `FORMATS` in `format.rs` | 10.3 |
| Running external programs | `run_command` in `converter.rs` | `std::process` docs |
| `Arc` and sharing across threads | `AppState` in `crates/server/src/lib.rs` | 16 |
| Async/await with tokio | the handlers in `crates/server/src/api.rs` | [Tokio tutorial](https://tokio.rs/tokio/tutorial) |
| Blocking work inside async | `spawn_blocking` in `api.rs` | Tokio docs |
| Implementing traits for other crates' types | `IntoResponse` and `From` for `ApiError` in `crates/server/src/error.rs` | 10 |
| Cargo workspaces | the root `Cargo.toml` | 14.3 |
| Build scripts | `crates/server/build.rs` | [Cargo book](https://doc.rust-lang.org/cargo/reference/build-scripts.html) |
| Threads and channels | `crates/desktop/src/main.rs` | 16 |

## Good first exercises

Each one is small and touches one or two concepts.

1. **Add a format alias.** Make `.oga` map to `ogg` in `normalize_ext`, and add a test. (match, tests)
2. **Add a converter.** Follow [ADDING_A_CONVERTER.md](ADDING_A_CONVERTER.md). `cwebp`, `libheif` (`heif-convert`) or `calibre` (`ebook-convert`) are good candidates. (traits)
3. **Sniff file contents.** Make `detect_format` also look at the first bytes of the file (`%PDF`, `\x89PNG`…). The [`infer`](https://docs.rs/infer) crate can help. (Option, crates)
4. **Multi-page PDFs.** In `imagemagick.rs`, render every page and zip the results. (std::fs, the `zip` crate)
5. **Conversion options.** Add an optional `quality` field to `Step` and pass it through the API. (structs, lifetimes, serde)

Tip: run `cargo clippy`. Its suggestions are a great Rust teacher.
