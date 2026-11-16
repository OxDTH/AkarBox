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
use crate::converter::{all_pairs, on_path, run_command, Converter, Edge, Step};
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

    fn convert(&self, step: &Step) -> Result<()> {
        run_command(Command::new("mytool").arg(step.input).arg(step.output))
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

**Choosing a cost:** costs add up along a route and the cheapest route wins. Use `1` for fast, lossless conversions and higher numbers for slow or lossy ones. LibreOffice uses `3` because it takes a second or two to start.

## 3. Register it

In [`crates/core/src/converters/mod.rs`](../crates/core/src/converters/mod.rs), add `mod mytool;`, then add `Box::new(mytool::MyTool)` to `builtin()`.

## 4. Test it

```bash
cargo test
cargo run -p akarbox-server
curl -F target=png -F file=@photo.heic localhost:8080/api/convert -o out.png
```

Set `RUST_LOG=debug` to see the exact commands that run. If your tool needs a new system package, add it to the `Dockerfile` and the README.
