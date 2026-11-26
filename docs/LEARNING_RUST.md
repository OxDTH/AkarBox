# Learning Rust with AkarBox

This guide is for people (like us!) who are learning Rust and want to use AkarBox to practise. The code is commented with beginner-friendly notes. This page tells you **what to read, in what order, and what to look for**.

You don't need FFmpeg, LibreOffice or any of the other tools to start. Install Rust from [rustup.rs](https://rustup.rs), clone the repo, and run:

```bash
cargo test -p akarbox-core
```

The planner tests use fake converters, and the tests that need real tools skip themselves when the tools are missing.

**Read alongside:** [The Rust Book](https://doc.rust-lang.org/book/) (free, online). The chapter numbers below refer to it. [Rustlings](https://github.com/rust-lang/rustlings) is great for small exercises.

---

## A reading path through the code

Each stop is one file, roughly easiest first. Read the file top to bottom, then try the small "try it" task, then run `cargo test` to check you didn't break anything.

### Stop 1: `crates/core/src/format.rs`: structs, enums, `match`, `Option`

The list of every format AkarBox knows.

- `enum Category` and `struct Format` are plain data types. Notice `#[derive(...)]`, which asks the compiler to write common code for you (Book ch. 5, 6).
- `normalize_ext` uses `match` on a string. The `_ =>` arm means "anything else".
- `format_from_name` uses the `?` operator on an `Option`: "if this is `None`, stop and return `None`" (ch. 6, 9.2).
- `detect_format` uses `let ... else`: "if this pattern doesn't match, run the `else` block, which must return".
- The `#[cfg(test)] mod tests` block at the bottom is where unit tests live (ch. 11).

**Try it:** make `.oga` map to `ogg` in `normalize_ext` and add an `assert_eq!` for it.

### Stop 2: `crates/core/src/sniff.rs`: bytes, slices and closures

Recognises a file from its first bytes ("magic numbers").

- `&[u8]` is a *slice*: a view into some bytes without owning them (ch. 4.3).
- `b"%PDF-"` is a byte-string literal. `h.starts_with(b"...")` compares the first bytes.
- `let at = |offset, magic| ...` is a *closure* (an inline function) that captures `h` (ch. 13.1).
- `h.get(a..b)` returns `None` instead of crashing when the range is out of bounds. This is a common Rust habit: prefer methods that return `Option` over ones that can panic.
- `haystack.windows(n).any(|w| w == needle)`: iterators chain small steps together (ch. 13.2).

**Try it:** the test `recognises_common_signatures` is a table of `(bytes, expected)` pairs. Add a row for a format that's missing, e.g. a GIF87a header.

### Stop 3: `crates/core/src/error.rs` and `options.rs`: `Result`, errors, serde

- Rust has no exceptions. Functions that can fail return `Result<T, Error>`.
- `thiserror` writes the boring `Display` code for our `Error` enum. `#[from]` lets `?` convert other error types into ours automatically (ch. 9).
- `Options` is turned from JSON into a struct by `serde`. Look at `#[serde(default, deny_unknown_fields)]`: missing fields get defaults, and typos become errors.
- `validate` contains a *generic* helper `fn check<T: PartialOrd + Display + Copy>`. The part after the colon is a *trait bound*: "T can be any type that can be compared, printed and copied" (ch. 10.1–10.2).

**Try it:** add a `max_fps: Option<u32>` option, validate it (1–120), and see which tests you need to update. Then remove it again (or keep it and use it in `ffmpeg.rs`!).

### Stop 4: `crates/core/src/converter.rs`: traits, lifetimes, running programs

- `trait Converter` is the plug-in interface every converter implements (ch. 10.2).
- `Send + Sync` on the trait means converters are safe to share between threads.
- `struct Step<'a>` holds *references* (`&'a Path`). The `'a` is a *lifetime*: it says "a `Step` can't outlive the paths it points to". The compiler checks this, so dangling pointers are impossible (ch. 10.3).
- `enum Produced { File, Pages(Vec<PathBuf>) }` is an enum where one variant carries data. That's how a converter says "I made one file" or "I made several".
- `run_command` runs an external program with `std::process::Command` and turns failure into an `Error`.

### Stop 5: `crates/core/src/converters/*.rs`: implementing a trait

Start with `pandoc.rs` (simplest), then `ffmpeg.rs`, `heif.rs`, and `imagemagick.rs` (most logic).

- `impl Converter for Pandoc { ... }` is how a type fulfils a trait.
- `ffmpeg.rs` shows `if let Some(h) = opts.video_height { ... }`: "if there is a value, name it `h` and use it".
- `heif.rs` has `find_map`, `filter` and `is_some_and`, all handy `Option`/iterator helpers.
- `imagemagick.rs` → `convert_pdf` renders every page, reads a directory, sorts the files and returns `Produced::Pages`.

### Stop 6: `crates/core/src/pdf.rs`: parsing user input safely

`parse_page_ranges` turns text like `"1-3, 5, 8-end"` into `[(1,3), (5,5), (8,10)]`.

- `split(',')`, `trim`, `split_once('-')` are the tools for parsing strings.
- `let (Some(start), Some(end)) = (start, end) else { return Err(...) }` destructures two options at once.
- Notice how every bad input returns a helpful `Err` instead of crashing. The tests at the bottom list the bad inputs we thought of.

**Try it:** add a new bad input to `rejects_bad_ranges` that you think might slip through. Does it? If it does, you found a bug, so fix it!

### Stop 7: `crates/core/src/registry.rs`: collections and an algorithm

The heart of AkarBox. It finds the cheapest chain of converters.

- `Vec<Box<dyn Converter>>` is a list of *different* types that all implement `Converter` (*trait objects*, ch. 18.2).
- `HashMap` stores the graph. `entry(..).or_default()` means "get it, or insert an empty one".
- `targets` is [Dijkstra's shortest-path algorithm](https://en.wikipedia.org/wiki/Dijkstra%27s_algorithm) using a `BinaryHeap`. `Reverse` turns the max-heap into a min-heap.
- `execute` uses `match` with *guards* (`Produced::Pages(pages) if pages.len() == 1 =>`). Arms are checked top to bottom.

### Stop 8: `crates/core/tests/real_tools.rs`: integration tests and macros

- Files in `tests/` are compiled as separate programs that can only use the crate's `pub` items, just like a real user.
- `macro_rules! require` is a small *macro*: code that writes code. `require!("qpdf", "convert")` expands to "return early unless these programs are installed" (ch. 20.5).

### Stop 9: `crates/server/src/`: async web server (do this last)

Async Rust is the hardest part. Read the [Tokio tutorial](https://tokio.rs/tokio/tutorial) first.

- `lib.rs`: `Arc` (shared ownership across threads, ch. 16.3), `Semaphore` (limits how many conversions run at once), and the router that maps URLs to functions.
- `api.rs`: each handler is an `async fn`. Arguments like `State(state)` and `Multipart` are *extractors*, and axum fills them in from the request.
- `blocking<T, F>` is a generic function with a `where` clause. It moves slow, blocking work (running FFmpeg) onto a separate thread pool so the web server stays responsive.
- `error.rs`: implementing a trait from another crate (`IntoResponse`) for our own type, and `From` so that `?` converts errors into HTTP responses.

### Stop 10: `crates/desktop/src/main.rs`: threads and channels

Starts the server on a background thread and sends the chosen port back over a channel (`mpsc`, ch. 16.2).

---

## What changed in v0.2, and why

We didn't review these changes line by line, so here is a map of each feature to the code that implements it. Reading one feature end to end is a great exercise.

| Feature (upstream issue) | Where to look | Rust ideas you'll meet |
|---|---|---|
| Detect formats from file contents (OxDTH/AkarBox#3) | `sniff.rs`, `detect_format` in `format.rs`, `/api/detect` in `server/src/api.rs` | byte slices, closures, `if`/`else if` chains |
| Multi-page PDF → images as a ZIP (OxDTH/AkarBox#6) | `convert_pdf` in `imagemagick.rs`, `execute` + `zip_files` in `registry.rs` | enums with data, `match` guards, `std::fs` |
| HEIC photos (OxDTH/AkarBox#8) | `converters/heif.rs`, `ftyp` in `sniff.rs` | `Option` combinators, parsing command output |
| Conversion options (OxDTH/AkarBox#5) | `options.rs`, `Step::options`, `ffmpeg.rs`, `imagemagick.rs` | serde, generics with trait bounds, `if let` |
| PDF merge & split (OxDTH/AkarBox#7) | `pdf.rs`, `pdf_merge` / `pdf_split` in `server/src/api.rs` | string parsing, `let else`, error messages |
| Batch upload + "Download all" (OxDTH/AkarBox#4) | `frontend/src/ConvertView.tsx` (TypeScript; the server didn't need changes) | — |

## How to check a change when you're still learning

You don't need to understand every line to review usefully.

1. **Run it.** `cargo test` and `cargo clippy --workspace --all-targets`. CI does this on every PR too.
2. **Read the tests first.** They say what the code is *supposed* to do, in plain examples.
3. **Read the doc comments** (`///`) above each function before its body.
4. **Pick one thing you don't understand** and look it up, or ask in the PR. "Why is this `Arc`?" is a great review comment.
5. **Try to break it.** Add a test with weird input (empty strings, huge numbers, a file with no extension). A failing test is a valuable contribution.
6. **Let `cargo clippy` teach you.** Its suggestions link to explanations.

## Exercises, from small to bigger

1. **Format alias:** `.oga` → `ogg` (Stop 1).
2. **New signature:** recognise a format in `sniff.rs` and add a test (Stop 2).
3. **Camera RAW photos** (OxDTH/AkarBox#11): add `cr2`, `nef`, `arw`, `dng` to `FORMATS` and to ImageMagick's inputs with cost 2. Check `convert -list format` shows them first. (traits, slices)
4. **Use an option in a new place:** make `max_width` also resize PDF page renders (Stop 5).
5. **Split into separate files:** add a `separate: bool` to PDF splitting that uses `qpdf --split-pages` and returns a ZIP (Stops 6 and 9).
6. **Compression mode** (OxDTH/AkarBox#10): allow `pdf → pdf` with Ghostscript's `-dPDFSETTINGS=/ebook`. You'll need to relax the "skip `x → x`" rule in `all_pairs` for that converter. (planner, Stop 7)
7. **Clipboard paste** (OxDTH/AkarBox#9) is TypeScript only: listen for `paste` in `ConvertView.tsx` and call `add()` with the pasted image files.
