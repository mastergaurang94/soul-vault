# Soul Vault — Quality Grades

Last measured: 2026-02-15

## Methodology

Grades are based on four signals per module:

1. Test coverage estimate: based on presence of `#[cfg(test)]` blocks in module files and existing integration coverage.
2. Lint compliance: architecture (`scripts/lint-architecture.sh`), unwrap/process-exit (`scripts/lint-unwrap.sh`), and Rust checks from `scripts/lint-all.sh`.
3. Doc comments: percentage of files starting with module docs (`//!`).
4. File size compliance: percentage of files at or under 200 lines (`scripts/lint-file-size.sh`).

Grade scale:

- `A`: strong on all four signals.
- `B`: mostly strong with one moderate gap.
- `C`: mixed quality with notable gaps.
- `D`: multiple major gaps needing targeted remediation.

## Lint Baseline (Current Codebase)

| Check | Result | Notes |
| --- | --- | --- |
| `scripts/lint-architecture.sh` | Fail | 2 violations (`src/core/pipeline.rs` and `src/tui/watcher.rs` importing `crate::extractors`) |
| `scripts/lint-file-size.sh` | Fail | 34 files over 200 lines |
| `scripts/lint-unwrap.sh` | Pass | No non-test `.unwrap()`; no `process::exit` outside `src/main.rs` |
| `cargo clippy --all-targets -- -D warnings` | Pass | Clean |
| `cargo fmt -- --check` | Pass | Clean |
| `scripts/lint-all.sh` | Fail | Fails due to architecture + file-size checks |

## Module Grades

| Module | Test Coverage Estimate | Lint Compliance | Doc Comments | File Size Compliance | Grade | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `types/` | High (1/1 files with tests) | Pass architecture/unwrap | 1/1 files documented | 0/1 files within limit | B | Strong core type safety, but oversized monolith |
| `ui/` | Medium (1/3 files with tests) | Pass architecture/unwrap | 2/3 files documented | 2/3 files within limit | C | Presentation layer mostly healthy, one oversized file |
| `vault/` | High (4/5 files with tests) | Pass architecture/unwrap | 4/5 files documented | 2/5 files within limit | C | Good correctness checks; file splitting needed |
| `auth/` | Low (0/1 files with tests) | Pass architecture/unwrap | 1/1 files documented | 0/1 files within limit | D | Missing tests and oversized auth module |
| `extractors/` | Medium (2/3 files with tests) | Pass architecture/unwrap | 2/3 files documented | 1/3 files within limit | C | Parsing is tested; very large extractor files |
| `adapters/` | High (5/5 files with tests) | Pass architecture/unwrap | 5/5 files documented | 0/5 files within limit | C | Strong test/doc discipline but all files exceed size target |
| `core/` | Medium (4/6 files with tests) | **Fail architecture** + pass unwrap | 5/6 files documented | 3/6 files within limit | D | Dependency-direction violation plus several oversized files |
| `tui/` | Low (2/17 files with tests) | **Fail architecture** + pass unwrap | 17/17 files documented | 8/17 files within limit | D | Excellent docs, but low tests, large files, and one dependency violation |
| `cli/` | Medium (4/11 files with tests) | Pass architecture/unwrap | 10/11 files documented | 2/11 files within limit | C | Good docs; significant file-size debt in command handlers |
| `main.rs` | Low (0/1 files with tests) | Pass architecture/unwrap | 1/1 documented | 1/1 within limit | B | Entry point is clean and bounded |

## Immediate Remediation Priorities

1. Remove architecture violations by moving extractor calls behind allowed `core/` boundaries.
2. Split top oversized files first: `src/extractors/chatgpt.rs`, `src/extractors/local.rs`, `src/cli/export.rs`, `src/cli/pull.rs`, `src/adapters/codex.rs`.
3. Add tests in `auth/` and more `tui/` page-level behavior coverage.
