# Language Architecture & Extension Guide

This document describes the polymorphic adapter architecture for programming languages in **Judge Ma Di** ([`src/judge/languages/`](../src/judge/languages/)), including trait contracts, default method behavior, and step-by-step guides for adding new execution adapters.

---

## 1. The `Language` Trait Contract

All programming languages implement the `Language` trait defined in [`src/judge/languages/mod.rs`](../src/judge/languages/mod.rs).

Rust default method implementations eliminate boilerplate—**adapters only implement what differs from the defaults**:

| Method | Default | Description |
| :--- | :--- | :--- |
| `name(&self)` | *Required* | Language identifier string (e.g., `"cpp"`, `"python"`). |
| `ext(&self)` | *Required* | Source file extension without leading dot (e.g., `"cpp"`, `"py"`). |
| `compiler(&self)` | `None` | Path to compiler binary. If `None`, compile step is skipped. |
| `compiler_flags(&self)` | `&[]` | Compiler arguments (e.g., `&["--std=c++17", "-O2"]`). |
| `compile_command(&self)` | Auto-derived | Constructs `<compiler> <flags...> source.<ext> -o <artifact>`. |
| `compiled_artifact(&self)` | `"source"` if compiled | Binary name produced by compiler and copied to run sandbox. |
| `run_command(&self)` | `["./source"]` | Command executed inside the runtime Isolate sandbox. |
| `custom_checker(&self)` | `None` | Optional custom validator override from `./checker/` (fallback to task manifest). |

---

## 2. Built-in Languages

| Key | Type | Compiler | Flags | Run Command |
| :--- | :--- | :--- | :--- | :--- |
| `cpp` | Compiled | `/usr/bin/g++` | `--std=c++17 -O2` | `./source` |
| `c` | Compiled | `/usr/bin/gcc` | `--std=c11 -O2` | `./source` |
| `python` | Interpreted | `/usr/bin/python3` (compileall) | `-m compileall -b` | `/usr/bin/python3 source.py` |

---

## 3. Adding a New Language (3 Steps)

### Step 1: Create Adapter (`src/judge/languages/<name>.rs`)

#### Pattern A: Standard Compiled Language (e.g., Rust)

Only provide `name`, `ext`, `compiler`, and `compiler_flags`. Compilation, artifacts, and `./source` execution are auto-derived:

```rust
use crate::judge::languages::Language;

#[derive(Debug, Default)]
pub struct Rust;

impl Rust {
    pub const fn new() -> Self { Self }
}

impl Language for Rust {
    fn name(&self) -> &'static str { "rust" }
    fn ext(&self) -> &'static str { "rs" }
    fn compiler(&self) -> Option<&'static str> { Some("/usr/bin/rustc") }
    fn compiler_flags(&self) -> &'static [&'static str] { &["-O"] }
}
```

#### Pattern B: Interpreted / Script Language (e.g., JavaScript)

No compiler hook needed. Only provide `name`, `ext`, and `run_command`:

```rust
use crate::judge::languages::Language;

#[derive(Debug, Default)]
pub struct JavaScript;

impl JavaScript {
    pub const fn new() -> Self { Self }
}

impl Language for JavaScript {
    fn name(&self) -> &'static str { "javascript" }
    fn ext(&self) -> &'static str { "js" }
    fn run_command(&self) -> Vec<String> {
        vec!["/usr/bin/node".into(), "source.js".into()]
    }
}
```

#### Pattern C: Custom Domain Adapter (e.g., SQL)

Override `run_command` and optionally specify `custom_checker` to enforce a specialized validator:

```rust
use crate::judge::languages::Language;

#[derive(Debug, Default)]
pub struct Sql;

impl Sql {
    pub const fn new() -> Self { Self }
}

impl Language for Sql {
    fn name(&self) -> &'static str { "sql" }
    fn ext(&self) -> &'static str { "sql" }
    fn run_command(&self) -> Vec<String> {
        vec!["/usr/bin/sqlite3".into(), "-init".into(), "source.sql".into()]
    }
    fn custom_checker(&self) -> Option<&'static str> {
        Some("rcmp4")
    }
}
```

---

### Step 2: Register in `src/judge/languages/mod.rs`

1. Declare module and export struct:
   ```rust
   pub mod rust;
   pub use rust::Rust;
   ```

2. Declare static singleton:
   ```rust
   pub static RUST: Rust = Rust::new();
   ```

3. Add branch to `get_language`:
   ```rust
   pub fn get_language(name: &str) -> Result<&'static dyn Language, AppError> {
       match name {
           "cpp" => Ok(&CPP),
           "c" => Ok(&C_LANG),
           "python" => Ok(&PYTHON),
           "rust" => Ok(&RUST),
           _ => Err(AppError::BadRequest("Unsupported Language".to_string())),
       }
   }
   ```

---

### Step 3: Toolchain & Verification

1. **Install Compiler / CLI**: Add binary to [`Dockerfile`](../Dockerfile) (e.g., `apt-get install -y rustc`).
2. **Add Unit Test**: In [`tests/judge_languages.rs`](../tests/judge_languages.rs):
   ```rust
   #[test]
   fn rust_adapter_metadata_and_commands() {
       let rust = get_language("rust").unwrap();
       assert_eq!(rust.name(), "rust");
       assert_eq!(rust.ext(), "rs");
       assert_eq!(rust.run_command(), vec!["./source"]);
   }
   ```
3. **Verify**:
   ```bash
   $ cargo test --test judge_languages
   ```

---

## 4. Upgrading or Changing Standards (e.g. C++17 to C++20)

### Option A: Change Default Standard for Existing Key

To bump the standard for all `cpp` submissions, edit `compiler_flags` in [`src/judge/languages/cpp.rs`](../src/judge/languages/cpp.rs):

```rust
// Before (C++17):
fn compiler_flags(&self) -> &'static [&'static str] {
    &["--std=c++17", "-O2"]
}

// After (C++20):
fn compiler_flags(&self) -> &'static [&'static str] {
    &["--std=c++20", "-O2"]
}
```

Ensure the host/container compiler version supports the standard (e.g., GCC 11+ for C++20, GCC 14+ for C++23).

### Option B: Support Multiple Versions Concurrently (e.g. `cpp17` alongside `cpp20`)

Define version-specific structs under `src/judge/languages/`:

```rust
// In src/judge/languages/cpp.rs:
pub struct Cpp20;

impl Language for Cpp20 {
    fn name(&self) -> &'static str { "cpp20" }
    fn ext(&self) -> &'static str { "cpp" }
    fn compiler(&self) -> Option<&'static str> { Some("/usr/bin/g++") }
    fn compiler_flags(&self) -> &'static [&'static str] { &["--std=c++20", "-O2"] }
}
```

Register both in [`src/judge/languages/mod.rs`](../src/judge/languages/mod.rs):

```rust
pub static CPP: Cpp = Cpp::new();       // default "cpp" (C++17)
pub static CPP20: Cpp20 = Cpp20::new(); // "cpp20" (C++20)

pub fn get_language(name: &str) -> Result<&'static dyn Language, AppError> {
    match name {
        "cpp" | "cpp17" => Ok(&CPP),
        "cpp20" => Ok(&CPP20),
        "c" => Ok(&C_LANG),
        "python" => Ok(&PYTHON),
        _ => Err(AppError::BadRequest("Unsupported Language".to_string())),
    }
}
```

### When `Dockerfile` Updates Are Required

Changing Rust code alone is not enough if the container's installed toolchain does not support the version:

1. **Newer Compilers (e.g., GCC 13/14 for C++23)**:
   The base image `debian:bookworm-slim` ships GCC 12 by default. If a new language standard requires features only in newer GCC:
   * Install specific version in [`Dockerfile`](../Dockerfile):
     ```dockerfile
     RUN apt-get update && apt-get install -y --no-install-recommends \
         g++-13 \
         ...
     ```
   * Update adapter `compiler()` to match the installed binary:
     ```rust
     fn compiler(&self) -> Option<&'static str> {
         Some("/usr/bin/g++-13")
     }
     ```

2. **Newer Runtimes (e.g., Python 3.12+, Node.js 20+)**:
   If upgrading interpreter versions, install the target version in [`Dockerfile`](../Dockerfile) and point `run_command()` to the corresponding path (e.g., `/usr/bin/python3.12`).

3. **Rebuild Image**:
   Whenever `Dockerfile` package dependencies change:
   ```bash
   $ docker compose build worker
   ```
