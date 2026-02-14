# Soma — Design Principles

## Code Organization

1. **~200 lines per file max.** When a file gets larger, split it. The codebase currently averages ~185 lines/file.

2. **Strict dependency direction.** `types/` is the leaf — everything can import it, it imports nothing. `core/` never imports `cli/`. See `ARCHITECTURE.md` for the full dependency graph.

3. **One module, one concern.** Each file has a clear, documented purpose (the `//!` doc comment at the top).

## Error Handling

4. **`anyhow::Result` everywhere.** No `unwrap()` on fallible operations in non-test code. No `std::process::exit()` in library code — only in `main.rs`.

5. **Error messages are instructions.** Every error tells the user both what happened AND what to do about it:
   ```rust
   #[error("Soma vault not initialized.\n      → Run `soma init` to create your vault.")]
   NotInitialized,
   ```

6. **Use `thiserror` for typed errors.** `SomaError` in `types/mod.rs` defines all domain errors with actionable messages.

## Validate at Boundaries

7. **All external data is validated at entry.** API responses go through `parser.rs` (handles malformed JSON, missing fields, markdown fencing). File reads go through `extractors/`. User input is validated by clap.

8. **Graceful degradation.** If the LLM returns garbage JSON, `parse_extraction_response` returns `ExtractedMemories::default()` instead of crashing. Warnings are logged, processing continues.

## Vault Files Are the Product

9. **Markdown files must be human-readable.** They should look good in any text editor, not just programmatic access. Every vault file has YAML frontmatter, proper headings, clean formatting.

10. **Dedup at every layer.** The merger deduplicates by normalized content. Vault writers skip content that already exists in the file. Source tracking prevents re-ingesting unchanged files.

## Testing

11. **Test what matters.** Core logic (processor, parser, merger, vault operations) needs thorough tests. CLI glue code and UI formatting are less critical.

12. **Unit tests live with the code.** Every module has a `#[cfg(test)] mod tests` block. Integration tests in `tests/` cover end-to-end flows.

13. **Zero clippy warnings.** Run `cargo clippy --all-targets` and fix everything.

## Naming & Style

14. **`rustfmt.toml` is law.** Edition 2021, max_width 100. Run `cargo fmt` before committing.

15. **ASCII slugs for filenames.** Topic and people names are slugified to ASCII (`slugify()` in `vault/write.rs`) — no Unicode in vault filenames.

16. **Section separators.** Use `// ─── Section Name ───` comments to organize code within files. Keeps large files scannable.

## Dependencies

17. **Minimal dependency surface.** Every crate in `Cargo.toml` earns its place. `reqwest` uses `rustls-tls` (no OpenSSL). Features are explicitly selected.

18. **Release profile is optimized.** LTO, single codegen unit, stripped — produces a ~4.3 MB binary.

## Inspired By

- [OpenAI "Harness Engineering"](https://openai.com/index/harness-engineering/) — AGENTS.md as entry point, structured architecture, error messages as remediation
