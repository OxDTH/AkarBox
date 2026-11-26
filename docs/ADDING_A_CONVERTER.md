# Adding a converter

A converter wraps one tool (usually a command-line program) and tells AkarBox which conversions it can perform. The planner automatically combines it with every other converter.

## 1. Make sure the formats exist

Formats live in [`crates/core/src/format.rs`](../crates/core/src/format.rs). Add a line to `FORMATS` if yours is missing:

```rust
f("heic", "HEIC image", Image),
```

## 2. Implement the `Converter` trait

Create `crates/core/src/converters/mytool.rs`:

```rust
use crate::converter::{all_pairs, on_path, run_command, Converter, Edge, Produced, Step};
use crate::Result;
use std::process::Command;

pub struct MyTool;

impl Converter for MyTool {
    fn id(&self) -> &'static str {
        "mytool"
    }

    fn is_available(&self) -> bool {
        on_path("mytool")
    }

    fn edges(&self) -> Vec<Edge> {
        // Every input format × every output format, cost 1.
        all_pairs(&["heic"], &["png", "jpg"], 1)
    }

    fn convert(&self, step: &Step) -> Result<Produced> {
        run_command(Command::new("mytool").arg(step.input).arg(step.output))?;
        Ok(Produced::File)
    }
}
```

`Step` gives you:

| Field      | Meaning                                                       |
|------------|---------------------------------------------------------------|
| `input`    | The file to read                                              |
| `output`   | Where to write the result (must exist when you return `Ok`)   |
| `from`/`to`| Extensions, useful when one converter handles many formats    |
| `work_dir` | A private scratch folder, deleted afterwards                  |
| `options`  | The user's settings (`quality`, `max_width`, `audio_bitrate_kbps`...). Apply the ones that make sense for your tool and ignore the rest. Only the last step of a route receives them. |

**Return value:** `Produced::File` when you wrote `step.output`. If the input naturally becomes several files (like one image per PDF page), write them into `work_dir` and return `Produced::Pages(paths)`. AkarBox zips them for the user.

**Choosing a cost:** costs add up along a route and the cheapest route wins. Use `1` for fast, lossless conversions and higher numbers for slow or lossy ones. LibreOffice uses `3` because it takes a second or two to start.

## 3. Register it

In [`crates/core/src/converters/mod.rs`](../crates/core/src/converters/mod.rs), add `mod mytool;`, then add `Box::new(mytool::MyTool)` to `builtin()`.

## 4. Test it

```bash
cargo test
cargo run -p akarbox-server
curl -F target=png -F file=@photo.heic localhost:8080/api/convert -o out.png
```

Set `RUST_LOG=debug` to see the exact commands that run. If your tool needs a new system package, add it to the `Dockerfile`, `.github/workflows/ci.yml` and the README.

Add a test to `crates/core/tests/real_tools.rs` that creates a small sample file, converts it, and checks the result. Start it with `require!("mytool");` so it skips itself on machines without the tool. `heic_converts_via_libheif` is a good example to copy.

**Also teach AkarBox to recognise the format.** If your format has a fixed signature at the start of the file, add it to `sniff()` in `crates/core/src/sniff.rs`, with a test case.
