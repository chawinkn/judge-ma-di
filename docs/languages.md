# Language Architecture & Extension Guide

A language in **Judge Ma Di** is a plain value, not a trait: a `Language` describes how to build a submission and what to run ([`src/judge/languages/mod.rs`](../src/judge/languages/mod.rs)). Adding one is one table entry.

---

## 1. The `Language` Value

```rust
pub struct Compile {
    pub cmd: &'static [&'static str], // run in the compile sandbox
    pub artifact: &'static str,       // file it produces, copied to the run sandbox
}

pub struct Language {
    pub name: &'static str,           // key stored in `submission.language`
    pub source_file: &'static str,    // filename the code is written to
    pub compile: Option<Compile>,     // None = nothing to build
    pub run: &'static [&'static str], // command run in the run sandbox
}
```

All languages live in the `LANGUAGES` slice; `get_language(name)` looks them up by `name` and returns `BadRequest("Unsupported Language")` otherwise.

If `compile` is set, the compile sandbox runs `cmd`, then `artifact` is copied into the run sandbox. A successful compile that produces no `artifact` is a misconfiguration and surfaces as `Judge Error`. `run` must execute the artifact (or `source_file` when there is no `compile`). The checker always comes from the task manifest.

---

## 2. Built-in Languages

| Key | Source | Compile | Artifact | Run |
| :--- | :--- | :--- | :--- | :--- |
| `cpp` | `source.cpp` | `/usr/bin/g++ --std=c++17 -O2 source.cpp -o source` | `source` | `./source` |
| `c` | `source.c` | `/usr/bin/gcc --std=c11 -O2 source.c -o source` | `source` | `./source` |
| `python` | `source.py` | `/usr/bin/python3 -m compileall -b -q source.py` | `source.pyc` | `/usr/bin/python3 source.pyc` |

---

## 3. Adding a New Language

### Step 1: Add an entry to `LANGUAGES`

Compiled (e.g., Rust):

```rust
Language {
    name: "rust",
    source_file: "source.rs",
    compile: Some(Compile {
        cmd: &["/usr/bin/rustc", "-O", "source.rs", "-o", "source"],
        artifact: "source",
    }),
    run: &["./source"],
},
```

Interpreted (e.g., JavaScript), no build step:

```rust
Language {
    name: "javascript",
    source_file: "source.js",
    compile: None,
    run: &["/usr/bin/node", "source.js"],
},
```

### Step 2: Toolchain & Verification

1. **Install Compiler / CLI**: Add binary to [`Dockerfile`](../Dockerfile) (e.g., `apt-get install -y rustc`).
2. **Add Unit Test**: In [`tests/judge_languages.rs`](../tests/judge_languages.rs), assert the new entry's source, compile, artifact and run. The consistency test over `LANGUAGES` (compile `cmd` builds `source_file`, `run` uses `artifact`) covers it automatically.
3. **Verify**:
   ```bash
   $ cargo test --test judge_languages
   ```

---

## 4. Upgrading or Changing Standards (e.g. C++17 to C++20)

### Option A: Change Default Standard for Existing Key

To bump the standard for all `cpp` submissions, edit the `cmd` of the `cpp` entry in [`src/judge/languages/mod.rs`](../src/judge/languages/mod.rs): replace `"--std=c++17"` with `"--std=c++20"`.

Ensure the host/container compiler version supports the standard (e.g., GCC 11+ for C++20, GCC 14+ for C++23).

### Option B: Support Multiple Versions Concurrently (e.g. `cpp17` alongside `cpp20`)

Add a second entry with its own `name` and the new flag; both can share `source_file` and `artifact`:

```rust
Language {
    name: "cpp20",
    source_file: "source.cpp",
    compile: Some(Compile {
        cmd: &["/usr/bin/g++", "--std=c++20", "-O2", "source.cpp", "-o", "source"],
        artifact: "source",
    }),
    run: &["./source"],
},
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
   * Point the entry's `compile.cmd` at the installed binary (e.g., `"/usr/bin/g++-13"`).

2. **Newer Runtimes (e.g., Python 3.12+, Node.js 20+)**:
   If upgrading interpreter versions, install the target version in [`Dockerfile`](../Dockerfile) and point `run` (and `compile.cmd`) to the corresponding path (e.g., `/usr/bin/python3.12`).

3. **Rebuild Image**:
   Whenever `Dockerfile` package dependencies change:
   ```bash
   $ docker compose build worker
   ```
