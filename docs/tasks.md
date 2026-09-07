# Task Specification & Configuration Guide

This document describes how tasks (problems) are configured, stored, and scored in **Judge Ma Di**, using the concrete `a_plus_b` fixture and subtask examples.

---

## 1. Directory Structure

Each task lives in a dedicated folder under `tasks/<task_id>/`:

```text
tasks/<task_id>/
├── manifest.json       # Problem configuration & judging limits
├── desc.pdf            # Problem statement PDF
├── testcases.zip       # Archive containing the testcase pairs
└── testcases/          # Extracted input and solution pairs
    ├── 1.in            # Testcase 1 input
    ├── 1.sol           # Testcase 1 expected output
    ├── 2.in
    ├── 2.sol
    ├── ...
    ├── N.in            # Testcase N input
    └── N.sol           # Testcase N expected output
```

### File Conventions
* **Input files (`<index>.in`)**: Plain text fed directly to the user process via host `stdin`. Numbered consecutively starting from `1` up to `num_testcases`.
* **Solution files (`<index>.sol`)**: Plain text expected output passed to the checker binary. Numbering matches `.in` files.
* **Problem statement (`desc.pdf`)**: Downloaded by users via `GET /api/tasks/:id/desc`.
* **Archive (`testcases.zip`)**: Downloaded via `GET /api/tasks/:id/testcases` and unpacked automatically on upload.

---

## 2. `manifest.json` Schema Reference

```json
{
  "time_limit": 0.3,
  "memory_limit": 16,
  "checker": "lcmp",
  "skip": false,
  "full_score": 100,
  "num_testcases": 10,
  "subtasks": []
}
```

| Field | Type | Description |
| :--- | :--- | :--- |
| `time_limit` | `float` | Maximum CPU execution time in seconds per testcase (enforced by Isolate `--time`). |
| `memory_limit` | `integer` | Maximum memory limit in megabytes per testcase (enforced by Isolate cgroups v2 `--cg-mem`). |
| `checker` | `string` | Binary name of the validator located in `./checker/` (e.g. `lcmp`, `wcmp`, `ncmp`). |
| `skip` | `boolean` | If `true`, stops evaluating subsequent testcases in a subtask immediately upon the first non-OK verdict. |
| `full_score` | `integer` | Total maximum score awarded for passing all testcases (typically `100`). |
| `num_testcases` | `integer` | Total count of testcases (`1` through `N`). All matching `.in` and `.sol` files must exist. |
| `subtasks` | `array` | List of subtask definitions. Empty `[]` indicates flat uniform scoring across all testcases. |

---

## 3. Full Example 1: Flat Scoring (`a_plus_b`)

`a_plus_b` is a standard introductory problem without subtask grouping. Every testcase is evaluated and scored independently.

### `tasks/a_plus_b/manifest.json`

```json
{
  "time_limit": 0.3,
  "memory_limit": 16,
  "checker": "lcmp",
  "skip": false,
  "full_score": 100,
  "num_testcases": 10,
  "subtasks": []
}
```

### Testcase Layout & Content

`tasks/a_plus_b/testcases/` contains 10 testcase pairs (`1.in` / `1.sol` through `10.in` / `10.sol`):

* **`tasks/a_plus_b/testcases/1.in`**:
  ```text
  -1000000000 
  -1000000000
  ```
* **`tasks/a_plus_b/testcases/1.sol`**:
  ```text
  -2000000000
  ```

* **`tasks/a_plus_b/testcases/3.in`**:
  ```text
  0 0
  ```
* **`tasks/a_plus_b/testcases/3.sol`**:
  ```text
  0
  ```

### Flat Scoring Mechanics

When `subtasks` is empty (`[]`):
1. **Per-Testcase Weight**: Each testcase is worth $\frac{\text{full\_score}}{\text{num\_testcases}}$ points (here: $100 / 10 = 10$ points each).
2. **Independent Evaluation**: If testcase 2 fails (e.g., Wrong Answer), testcases 3 through 10 still run.
3. **Total Score**: Sum of points earned on passing testcases.

### Reference Solutions (100 / 100)

**C++ (`cpp`)**:
```cpp
#include <iostream>

int main() {
    std::ios_base::sync_with_stdio(false);
    std::cin.tie(NULL);
    long long a, b;
    if (std::cin >> a >> b) {
        std::cout << a + b << "\n";
    }
    return 0;
}
```

**Python (`python`)**:
```python
import sys

def main():
    tokens = sys.stdin.read().split()
    if len(tokens) >= 2:
        print(int(tokens[0]) + int(tokens[1]))

if __name__ == "__main__":
    main()
```

---

## 4. Full Example 2: Subtask Scoring (`tasks/0`)

For Olympiad-style problems, testcases are partitioned into subtasks with specific constraints and all-or-nothing scoring.

### `tasks/0/manifest.json`

```json
{
  "time_limit": 0.5,
  "memory_limit": 1024,
  "checker": "lcmp",
  "skip": true,
  "full_score": 100,
  "num_testcases": 10,
  "subtasks": [
    {
      "full_score": 20,
      "num_testcases": 2
    },
    {
      "full_score": 30,
      "num_testcases": 3
    },
    {
      "full_score": 50,
      "num_testcases": 5
    }
  ]
}
```

### Subtask Structure

| Subtask Index | Testcase Range | Full Score | Scoring Model |
| :--- | :--- | :--- | :--- |
| **Subtask 1** | Testcases 1 – 2 | 20 points | All-or-nothing (both tests must pass) |
| **Subtask 2** | Testcases 3 – 5 | 30 points | All-or-nothing (all 3 tests must pass) |
| **Subtask 3** | Testcases 6 – 10 | 50 points | All-or-nothing (all 5 tests must pass) |

### Subtask Scoring & `skip` Mechanics

1. **Sequential Mapping**: Testcase indices map sequentially to subtasks. Subtask 1 consumes tests 1..2, Subtask 2 consumes tests 3..5, Subtask 3 consumes tests 6..10.
2. **All-or-Nothing Score**: If any testcase within a subtask fails (Wrong Answer, TLE, MLE, or RE), the score for that entire subtask is `0`.
3. **Short-Circuit Evaluation (`skip: true`)**:
   - If testcase 1 passes but testcase 2 produces a Wrong Answer, Subtask 1 fails.
   - Because `skip: true`, testcases within a failed subtask do not continue unnecessarily; any remaining testcases in that subtask are marked as `Skipped`.
   - Evaluation then advances to the next subtask (Subtask 2 begins at testcase 3).

---

## 5. Checker Reference

The `checker` field references compiled binaries in `./checker/` built from [testlib](https://github.com/MikeMirzayanov/testlib). The worker invokes:

```bash
./checker/<name> <input_file> <user_output_file> <solution_file>
```

| Checker | Validation Behavior | Typical Use Case |
| :--- | :--- | :--- |
| `lcmp` | Line-by-line comparison, ignoring trailing whitespace and newline differences. | Most standard tasks, text output, multi-line solutions. |
| `wcmp` | Whitespace-delimited token comparison (words/numbers). Ignores arbitrary formatting spacing. | Problems where output format allows any whitespace separator. |
| `ncmp` | Integer token comparison. | Problems with integer sequences or single numbers. |
| `rcmp4` | Floating point comparison with precision tolerance $\epsilon \le 10^{-4}$. | Numerical approximations. |
| `rcmp6` | Floating point comparison with precision tolerance $\epsilon \le 10^{-6}$. | Standard floating point problems. |
| `rcmp9` | Floating point comparison with precision tolerance $\epsilon \le 10^{-9}$. | High-precision geometry / math problems. |
| `yesno` | Case-insensitive match for `YES` / `NO`. | Decision problems accepting `Yes`, `yes`, `YES`, etc. |
| `fcmp` | Strict full-file binary diff. | Exact formatting / byte-exact output requirements. |

---

## 6. How to Add or Upload a Task

### Method A: Via HTTP REST API

Upload task assets directly to a running `judge-api` or monolith instance:

```bash
# Package testcases into a zip
cd tasks/a_plus_b/testcases && zip -r ../testcases.zip . && cd ../../..

# Upload manifest, description, and testcase archive
curl -X POST http://localhost:5000/api/tasks/a_plus_b \
  -F "manifest.json=@tasks/a_plus_b/manifest.json" \
  -F "desc.pdf=@tasks/a_plus_b/desc.pdf" \
  -F "testcases.zip=@tasks/a_plus_b/testcases.zip"
```

The API creates `tasks/a_plus_b/`, writes the files, and automatically unzips `testcases.zip` into `tasks/a_plus_b/testcases/`.

### Method B: Via Database Registration

To make the task visible to the frontend or contest system, register it in PostgreSQL:

```sql
INSERT INTO task (id, title, full_score, private)
VALUES ('a_plus_b', 'A + B Problem', 100, FALSE)
ON CONFLICT (id) DO UPDATE SET
  title = EXCLUDED.title,
  full_score = EXCLUDED.full_score,
  private = EXCLUDED.private;
```

### Method C: Direct Filesystem Mount

In Docker or local development, place or mount the task directory directly at:
```text
tasks/<task_id>/
```
The worker and API read directly from this path.
