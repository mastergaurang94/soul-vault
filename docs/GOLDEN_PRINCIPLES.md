# Soul Vault — Golden Principles
Last updated: 2026-02-15

These are mechanical rules. They are intentionally opinionated and should be enforced continuously.

## 1) Reuse Shared Utilities

- Prefer existing helpers in shared modules before adding new local helpers.
- If similar logic appears in 2+ files, extract to one shared utility.
- No copy-paste variants of parsing, path handling, slugging, or markdown formatting.

## 2) Validate At Boundaries

- Parse and validate external input at entry points (`cli`, `extractors`, adapters, file I/O).
- Internal code consumes typed, validated structures only.
- Never probe untrusted data YOLO-style (`unwrap`, unchecked indexing, ad-hoc string slicing).

## 3) Errors Must Teach Recovery

- Every user-facing error must state what failed and what to do next.
- Include exact command or path when applicable.
- Avoid ambiguous messages like "failed" without remediation.

## 4) File Length Budget

- Keep source files under 200 lines when practical.
- If a file crosses 200 lines, split by responsibility, not by arbitrary chunks.
- Temporary exceptions must be tracked in `docs/STATUS.md` with an owner.

## 5) ASCII Slugs For Filenames

- Generated or normalized filenames must be ASCII-only slug form.
- Use existing slug utilities; do not invent alternate slug rules.
- Example: `Ren\u00e9 Descartes` -> `rene-descartes.md`.

## 6) Section Separators In Longer Files

- Use `// ─── Section Name ───` separators in files with multiple logical blocks.
- Keep section names concise and stable.
- Do not over-segment tiny modules.

## 7) Test Naming Conventions

- Unit tests use `snake_case` and describe behavior, e.g. `parses_markdown_with_frontmatter`.
- Regression tests include issue context, e.g. `regression_missing_timestamp_defaults_to_none`.
- Integration tests should read like user workflows, e.g. `import_then_export_bundle`.

## 8) Commit Message Format

- Format: `<type>: <summary>`.
- Allowed types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`.
- Summary is imperative, concise, and scoped to one logical change.

## 9) Import Ordering

- Standard library imports first.
- Third-party crates second.
- Internal crate imports third.
- Keep each group alphabetized and separated by one blank line.

## 10) Module Docs Required

- Every `src/**/*.rs` file starts with a `//!` module doc comment.
- The module doc states responsibility in 1-2 lines.
- No file is considered complete without module docs.
