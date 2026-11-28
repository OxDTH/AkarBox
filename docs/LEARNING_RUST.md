# Learning Rust with AkarBox: from zero to v0.1.0

This guide assumes you have **never written Rust**. It's a study plan for the AkarBox team (and anyone else) to go from "what is `cargo`?" to "I can confidently change and review AkarBox code" before we release v0.1.0.

It's built around one idea: **read a little theory, then immediately find it in real code you care about.** AkarBox is commented for beginners, and each week below points at the exact files to read.

> **Time:** about 6 weeks at 1–2 hours a day. Going slower is fine. Skipping the exercises is not, because typing code is how it sticks.

---

## Contents

1. [Before you start (day 0)](#1-before-you-start-day-0)
2. [How to study](#2-how-to-study)
3. [The 6-week plan](#3-the-6-week-plan)
4. [Ready for v0.1.0? A checklist](#4-ready-for-v010-a-checklist)
5. [Map of the code](#5-map-of-the-code)
6. [Rust cheat sheet, with AkarBox examples](#6-rust-cheat-sheet-with-akarbox-examples)
7. [How to test and review a PR while learning](#7-how-to-test-and-review-a-pr-while-learning)
8. [Getting unstuck](#8-getting-unstuck)

---

## 1. Before you start (day 0)

### Install

1. **Rust:** follow [rustup.rs](https://rustup.rs). It installs `rustc` (the compiler) and `cargo` (the build tool). Check with `cargo --version`.
2. **An editor with Rust support:** [VS Code](https://code.visualstudio.com/) + the **rust-analyzer** extension. It shows types as you hover, underlines errors before you compile, and lets you jump to definitions (F12). This alone makes learning twice as fast.
3. **Git**, to get the code:

```bash
git clone https://github.com/OxDTH/AkarBox
```
```bash
cd AkarBox && cargo test -p akarbox-core
```

The first build downloads libraries and takes a few minutes. When you see `test result: ok`, you're set up. **You don't need FFmpeg, LibreOffice, Node.js or anything else** to learn: tests that need those tools skip themselves.

### Words you'll see everywhere

| Word | Meaning |
|---|---|
| **crate** | A Rust package (a library or a program). AkarBox has three: `akarbox-core`, `akarbox-server`, `akarbox-desktop`. |
| **cargo** | The tool that builds, tests and runs crates. `cargo build`, `cargo test`, `cargo run`. |
| **workspace** | Several crates in one repo, built together. See the root `Cargo.toml`. |
| **module** | A file or folder of code inside a crate (`mod sniff;` pulls in `sniff.rs`). |
| **dependency** | A library we use, listed in `Cargo.toml` and downloaded from [crates.io](https://crates.io). |
| **compiler error** | Rust checks a lot *before* running. Errors are normal and helpful, not a sign you're bad at this. |

### The four commands you'll use daily

```bash
cargo test -p akarbox-core            # run the engine's tests
```
```bash
cargo test -p akarbox-core sniff      # only tests with "sniff" in their name
```
```bash
cargo clippy --workspace --all-targets   # extra advice ("lints") about your code
```
```bash
cargo fmt --all                        # auto-format the code
```

---

## 2. How to study

- **Read → type → break → fix.** Read a Book section, type its example (don't paste), change something until it breaks, read the error, fix it.
- **One AkarBox task per week.** Each week ends with a small, real change. Do it on a branch, run the tests, and open a PR if you like. That's practice for the real workflow.
- **Teach it back.** Once a week, each person explains one file to the others in 5 minutes. If you can't explain it, you've found what to reread.
- **Keep a "confusions" list.** Write down everything that confuses you. Many will resolve themselves by week 4. Bring the rest to the team.

**Main resources** (all free):

- 📘 [The Rust Programming Language](https://doc.rust-lang.org/book/) ("the Book"). Chapter numbers below refer to it.
- 🏋️ [Rustlings](https://github.com/rust-lang/rustlings): small exercises that fail until you fix them. Install once with `cargo install rustlings`, then run `rustlings` inside a folder created by `rustlings init`.
- 🔎 [Rust by Example](https://doc.rust-lang.org/rust-by-example/): short examples, good for quick lookups.

---

## 3. The 6-week plan

Every week has the same shape: **Learn** (Book + Rustlings), **Read in AkarBox**, **Do** (a small real task), **Check yourself** (answers hidden; click to reveal).

### Week 1: First programs, variables, functions

**Learn:** Book ch. 1, 2, 3. Rustlings: `intro`, `variables`, `functions`, `if`, `primitive_types`.

**Read in AkarBox:** `crates/core/src/format.rs`
- `FORMATS` is a big table (a *slice*) of every format AkarBox knows.
- `normalize_ext` lowercases an extension and maps aliases (`jpeg` → `jpg`) using `match`.
- At the bottom, `#[cfg(test)] mod tests` contains tests. Each `#[test]` function is one test.

**Do:** make `.oga` (an Ogg audio file) be recognised as `ogg`.
1. Add `"oga" => "ogg".into(),` to `normalize_ext`.
2. Add `assert_eq!(format_from_name("song.oga").unwrap().ext, "ogg");` to the test `detects_aliases_and_case`.
3. Run `cargo test -p akarbox-core format`.

**Check yourself:**

<details><summary>What's the difference between <code>let x = 5;</code> and <code>let mut x = 5;</code>?</summary>

Variables are immutable by default. `mut` allows changing `x` later. Rust makes you opt in so accidental changes are caught.
</details>

<details><summary>In <code>normalize_ext</code>, what does the <code>_ =&gt; ext</code> arm do?</summary>

`_` matches anything not matched above, so unknown extensions are returned unchanged. `match` must cover every possible value, and `_` is how you say "everything else".
</details>

### Week 2: Ownership and borrowing (the big one)

**Learn:** Book ch. 4 (read it twice, seriously), ch. 5. Rustlings: `vecs`, `move_semantics`, `structs`, `strings`.

Ownership is the idea that makes Rust different: every value has one owner, and other code can *borrow* it (`&value`) without taking it. It's how Rust is memory-safe without a garbage collector.

**Read in AkarBox:**
- `crates/core/src/sniff.rs`. `sniff(header: &[u8])` *borrows* a slice of bytes: it reads them but doesn't own them, so the caller keeps its data. Notice `h.get(a..b)`, which returns `None` instead of crashing if the range is out of bounds.
- `crates/core/src/options.rs`. `struct Options` groups all user settings, and `#[derive(...)]` asks the compiler to write common code (copying, comparing, JSON conversion).

**Do:** in `sniff.rs`, the test `recognises_common_signatures` is a table of `(bytes, expected format)`. Add a row for a GIF87a header (`b"GIF87a...."`, `"gif"`). Then add a row that should *fail* and watch the test fail, so you know the test really checks something. Remove it again.

**Check yourself:**

<details><summary>Why does <code>sniff</code> take <code>&amp;[u8]</code> instead of <code>Vec&lt;u8&gt;</code>?</summary>

`&[u8]` borrows the bytes, so the caller doesn't have to give them away or copy them. A `Vec<u8>` parameter would *move* ownership into the function. Borrowing is the default choice when a function only needs to read.
</details>

<details><summary>What's the difference between <code>String</code> and <code>&amp;str</code>?</summary>

`String` owns its text (it can grow and change). `&str` is a borrowed view into text owned by someone else. `&'static str` (used in `FORMATS`) is text baked into the program, which lives forever.
</details>

### Week 3: Enums, `Option`, `match` and errors

**Learn:** Book ch. 6, 8, 9. Rustlings: `enums`, `options`, `hashmaps`, `error_handling`.

Rust has no `null` and no exceptions. "Maybe a value" is `Option<T>` (`Some(x)` or `None`); "success or error" is `Result<T, E>` (`Ok(x)` or `Err(e)`). The `?` operator means: "if this is an error (or `None`), return it right now".

**Read in AkarBox:**
- `crates/core/src/error.rs`. One `enum Error` lists everything that can go wrong. `#[from]` lets `?` convert other errors into ours automatically.
- `detect_format` in `format.rs`. `let Some(x) = ... else { return ... };` is "unwrap this or bail out".
- `crates/core/src/pdf.rs`, `parse_page_ranges`. It turns `"1-3, 5, 8-end"` into a list of ranges, returning a helpful `Err` for every kind of bad input.
- `enum Compression` in `options.rs`, an enum with three plain variants.

**Do:** try to break `parse_page_ranges`. Think of a weird input (`" , 3"`? `"2-2"`? `"end-1"`?), add it to `rejects_bad_ranges` or `parses_page_ranges`, and run the tests. If you find an input that's handled wrongly, congratulations, you found a bug: fix it and keep the test.

**Check yourself:**

<details><summary>What does the <code>?</code> in <code>std::fs::metadata(path)?.len()</code> do?</summary>

`metadata` returns a `Result`. If it's `Err`, `?` returns that error from the current function immediately (converting it to our `Error` type via `From`). If it's `Ok(meta)`, it unwraps to `meta` and the code continues with `.len()`.
</details>

<details><summary>Why is <code>Option</code> safer than <code>null</code>?</summary>

The compiler won't let you use an `Option<T>` as a `T` until you've handled the `None` case (with `match`, `if let`, `?`, `unwrap_or`...). Forgetting a null check becomes a compile error, not a crash.
</details>

### Week 4: Traits, generics, closures, iterators, lifetimes

**Learn:** Book ch. 10, 13, and 18.2 (trait objects). Rustlings: `generics`, `traits`, `lifetimes`, `iterators`.

This week explains AkarBox's core design: **every converter implements one trait.**

**Read in AkarBox** (in this order):
1. `crates/core/src/converter.rs`: `trait Converter` (the interface), `struct Step<'a>` (holds *borrowed* paths; `'a` is a lifetime saying "a Step can't outlive the paths it points to"), and `enum Produced` (one file or several).
2. `crates/core/src/converters/pandoc.rs`: the simplest converter. Find `impl Converter for Pandoc`.
3. `raw.rs`: an enum (`Tool`) whose variants carry data, and `match` on it.
4. `ghostscript.rs`: tries compression profiles in a loop until the PDF is small enough.
5. `imagemagick.rs`: `fit_to_size` uses a *binary search* over quality and a *closure* (`try_with`) to keep the code short.
6. `ffmpeg.rs`: `HwEncoder` picks a GPU encoder; `convert` falls back to the CPU if the GPU fails.
7. `crates/core/src/registry.rs`: `Vec<Box<dyn Converter>>` holds different converter types in one list. `targets` finds the cheapest chain of converters with Dijkstra's algorithm.

**Do:** add a tiny converter. A good first one: **`cwebp`** (Google's WebP encoder; `sudo apt install webp`). Follow [ADDING_A_CONVERTER.md](ADDING_A_CONVERTER.md): an edge `png -> webp` with cost 1, `convert` runs `cwebp input -o output`. Then check with `cargo test` that the planner prefers it over ImageMagick when costs tie, or doesn't, and work out why.

**Check yourself:**

<details><summary>What does <code>Box&lt;dyn Converter&gt;</code> mean?</summary>

"Some value, of a type we don't know at compile time, that implements `Converter`, stored on the heap." It lets one `Vec` hold `Ffmpeg`, `Pandoc`, `Raw`... side by side. Calls go through a small lookup table at runtime ("dynamic dispatch").
</details>

<details><summary>In <code>fn check&lt;T: PartialOrd + Display + Copy&gt;</code> (options.rs), what is <code>T</code>?</summary>

A generic type parameter: the function works for *any* type `T` that can be compared (`PartialOrd`), printed (`Display`) and copied (`Copy`). It's used for both `u8` and `u32` values without writing the function twice.
</details>

<details><summary>Why does <code>Converter</code> require <code>Send + Sync</code>?</summary>

The web server runs many conversions at the same time on different threads, all sharing the same converters. `Send + Sync` is the compiler-checked promise that this is safe.
</details>

### Week 5: Modules, tests, smart pointers, threads

**Learn:** Book ch. 7, 11, 15 (skim), 16. Rustlings: `modules`, `tests`, `smart_pointers`, `threads`.

**Read in AkarBox:**
- `crates/core/src/lib.rs`: `mod` declares modules and `pub use` re-exports what other crates may use.
- `crates/core/tests/real_tools.rs`: *integration tests*, which use the crate like an outsider would. `macro_rules! require` is a small macro that skips a test when a tool isn't installed. `tests/fixtures/tiny.dng` is a 6 KB camera RAW file made by `make_tiny_dng.py`.
- `crates/server/src/lib.rs`: `Arc<Registry>` shares one registry across all requests (ch. 16.3), and a `Semaphore` limits how many conversions run at once.
- `crates/desktop/src/main.rs`: starts the server on a background thread and sends the port back over a channel (ch. 16.2).

**Do:** write an integration test. Copy `heic_converts_via_libheif` in `real_tools.rs` and adapt it: for example, check that converting a WAV to FLAC and back keeps the duration, using `ffprobe`. Make it skip with `require!` when tools are missing.

### Week 6: Async Rust and the web server

**Learn:** Book ch. 17 (async), then the [Tokio tutorial](https://tokio.rs/tokio/tutorial) up to "Shared state".

**Read in AkarBox:** follow **one upload from browser to download**:
1. `frontend/src/api.ts` → `convert()` sends a form with `target`, `options` and `file` to `POST /api/convert`.
2. `crates/server/src/lib.rs` → `router()` maps that URL to `api::convert`.
3. `crates/server/src/api.rs` → `convert()`: `save_upload` streams the file to a temp folder and `detect_format` recognises it. `registry.plan` picks the route, and `blocking(...)` runs the conversion off the async threads.
4. `crates/core/src/registry.rs` → `execute()` runs each step; `zip_files` bundles multi-page output.
5. Back in `api.rs`, `download()` builds the response, and the temp folder is deleted automatically when `dir` goes out of scope (that's ownership again!).

**Do:** run the server with logging and watch a conversion happen:

```bash
RUST_LOG=debug cargo run -p akarbox-server
```

In another terminal (any PNG works):

```bash
curl -F target=jpg -F 'options={"target_size_kb":50}' -F file=@some.png localhost:8080/api/convert -o out.jpg
```

Read the log lines and match each one to the code that printed it.

**Check yourself:**

<details><summary>Why does <code>api.rs</code> use <code>spawn_blocking</code> for conversions?</summary>

Async tasks share a few threads. A conversion waits seconds or minutes for FFmpeg or LibreOffice, and doing that on an async thread would freeze other requests. `spawn_blocking` moves the waiting onto a separate pool of threads meant for blocking work.
</details>

<details><summary>When is the uploaded file deleted?</summary>

`let dir = tempfile::tempdir()?` creates the folder. When the `convert` function ends, successfully or with an error, `dir` is dropped and `TempDir`'s `Drop` implementation deletes the folder. No cleanup code is needed.
</details>

---

## 4. Ready for v0.1.0? A checklist

Before we tag the release, each of us should be able to tick these:

- [ ] I can build and run the tests (`cargo test`) and explain what a failing test is telling me.
- [ ] I can explain, in my own words, how a file goes from upload to download (week 6).
- [ ] I can explain why `md -> png` takes three steps, and which code decides that (`registry.rs`).
- [ ] I've made at least one change myself: an alias, a sniff signature, a test or a converter.
- [ ] I've read one PR and left at least one real question or comment on it.
- [ ] I know where to look when a conversion fails (`RUST_LOG=debug`, then `run_command` errors).

And for the project:

- [ ] The draft PR's manual test list has been run on a real machine (see the PR description).
- [ ] `docker compose up -d` builds and works on a real machine.
- [ ] The desktop app has been tried on at least one OS.

---

## 5. Map of the code

| File | What it does | Rust ideas |
|---|---|---|
| `crates/core/src/format.rs` | The list of formats; name → format | structs, enums, `match`, `Option` |
| `crates/core/src/sniff.rs` | Recognise files by their first bytes | slices, byte strings, closures |
| `crates/core/src/options.rs` | User settings, compression levels, validation | serde, generics, enums |
| `crates/core/src/error.rs` | Every error that can happen | `Result`, `thiserror`, `From` |
| `crates/core/src/converter.rs` | The `Converter` trait and helpers | traits, lifetimes, default methods |
| `crates/core/src/converters/pandoc.rs` | Markdown, HTML, DOCX, EPUB… | implementing a trait |
| `crates/core/src/converters/libreoffice.rs` | Office documents | `Command`, file renames |
| `crates/core/src/converters/imagemagick.rs` | Images, PDF pages, target file size | binary search, closures, `Copy` structs |
| `crates/core/src/converters/ghostscript.rs` | PDF compression | loops, constant tables of tuples |
| `crates/core/src/converters/ffmpeg.rs` | Audio/video, compression, GPU encoding | enums with methods, fallbacks |
| `crates/core/src/converters/heif.rs` | iPhone HEIC photos | `Option` combinators |
| `crates/core/src/converters/raw.rs` | Camera RAW | enums carrying data, `Stdio` |
| `crates/core/src/pdf.rs` | Merge and split PDFs | string parsing, `let else` |
| `crates/core/src/registry.rs` | Finds routes, runs them, zips pages | `HashMap`, `BinaryHeap`, trait objects |
| `crates/core/tests/real_tools.rs` | Tests with the real tools | integration tests, macros |
| `crates/server/src/lib.rs` | Web server setup, routes | `Arc`, `Semaphore`, axum |
| `crates/server/src/api.rs` | The HTTP endpoints | async/await, extractors, generics |
| `crates/server/src/error.rs` | Errors → HTTP responses | implementing foreign traits |
| `crates/desktop/src/main.rs` | Desktop app | threads, channels |
| `frontend/src/` | The web UI (TypeScript + React, not Rust) | — |

### Which code implements which feature

| Feature (upstream issue) | Where |
|---|---|
| Detect format from bytes (#3) | `sniff.rs`, `detect_format`, `/api/detect` |
| Batch + Download all (#4) | `frontend/src/BatchView.tsx` |
| Options: quality, size, bitrate (#5) | `options.rs`, `ffmpeg.rs`, `imagemagick.rs`, `frontend/src/OptionsPanel.tsx` |
| Multi-page PDF → ZIP (#6) | `imagemagick.rs` (`convert_pdf`), `registry.rs` (`zip_files`) |
| PDF merge & split (#7) | `pdf.rs`, `api.rs` (`pdf_merge`, `pdf_split`), `frontend/src/PdfStudio.tsx` |
| HEIC (#8) | `heif.rs` |
| Paste from clipboard (#9) | `frontend/src/App.tsx` (`onPaste`) |
| Compress in the same format (#10) | `same_format` edges, `Compression`, `ghostscript.rs`, `registry.rs` (`targets`) |
| Camera RAW (#11) | `raw.rs`, RAW signatures in `sniff.rs`, `tests/fixtures/tiny.dng` |
| GPU video encoding (#12) | `ffmpeg.rs` (`HwEncoder`, `probe`, fallback in `convert`) |
| Image under N KB (#13) | `imagemagick.rs` (`fit_to_size`) |
| PDF under N KB (#14) | `ghostscript.rs` (profile loop) |
| New UI (#16) | `frontend/src/Home.tsx`, `SingleWorkspace.tsx`, `Preview.tsx`, `presets.ts`, `theme.ts` |

---

## 6. Rust cheat sheet, with AkarBox examples

| You see | It means | Example in AkarBox |
|---|---|---|
| `let x = ...;` / `let mut x` | variable / changeable variable | everywhere |
| `&x`, `&mut x` | borrow (read-only / read-write) | `sniff(&header)` |
| `Option<T>`: `Some(v)` / `None` | maybe a value | `Format::get` |
| `Result<T, E>`: `Ok(v)` / `Err(e)` | success or error | every `convert` |
| `?` | return early on `Err`/`None` | `std::fs::copy(a, b)?;` |
| `match x { A => .., _ => .. }` | branch on a value's shape | `normalize_ext` |
| `if let Some(v) = x { .. }` | "if there's a value, use it" | `ffmpeg.rs` options |
| `let Some(v) = x else { return .. };` | unwrap or leave | `detect_format` |
| `struct S { a: u32 }` | a record type | `Options`, `Step` |
| `enum E { A, B(u32) }` | one of several shapes | `Produced`, `Compression`, `Tool` |
| `impl S { fn f(&self) }` | methods on a type | `impl Ffmpeg` |
| `trait T { fn f(&self); }` | an interface | `Converter` |
| `impl T for S { .. }` | S fulfils interface T | `impl Converter for Pandoc` |
| `Box<dyn T>` | any type implementing T | `Registry::converters` |
| `Vec<T>`, `HashMap<K, V>` | list, dictionary | `Registry::graph` |
| `\|x\| x + 1` | closure (inline function) | `try_with` in `imagemagick.rs` |
| `.iter().map(..).filter(..).collect()` | iterator pipeline | `all_pairs` |
| `'a`, `'static` | lifetimes: how long a borrow is valid | `Step<'a>`, `FORMATS` |
| `#[derive(Debug, Clone)]` | auto-generate common code | most structs |
| `name!(...)` | macro call | `format!`, `vec!`, `require!` |
| `async fn` / `.await` | non-blocking function / wait for it | `api.rs` |
| `Arc<T>` | shared ownership across threads | `AppState` |

---

## 7. How to test and review a PR while learning

You don't need to understand every line to review usefully.

1. **Run it.** `cargo test` and `cargo clippy --workspace --all-targets`. CI also runs these on every PR.
2. **Try it like a user.** Start the app, follow the PR's "how to test" list, and try to break it with weird files (empty, huge, wrong extension).
3. **Read the tests first.** They show what the code is *supposed* to do, with plain examples.
4. **Read the `///` doc comments** above functions before reading their bodies.
5. **Ask one question.** "Why is this an `Arc`?" or "What happens if this file is empty?" are great review comments. Questions catch real bugs.
6. **Let `cargo clippy` teach you.** Its warnings link to explanations.

---

## 8. Getting unstuck

- **Read the whole compiler error.** Rust's errors usually say exactly what's wrong and often suggest the fix. For details, run `rustc --explain E0382` (with your error code).
- **Hover in your editor.** rust-analyzer shows the type of anything you hover over. Most confusion is "what type is this?".
- **Make it smaller.** Copy the confusing code into a scratch project (`cargo new scratch`) and play with it there.
- **Search the Book's index** or [Rust by Example](https://doc.rust-lang.org/rust-by-example/).
- **Ask the team or the community:** the [Rust users forum](https://users.rust-lang.org/) is famously friendly to beginners.

Welcome aboard. Every Rust developer went through the "fighting the borrow checker" phase, and it passes. 🦀
