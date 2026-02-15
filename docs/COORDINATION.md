# Soul Vault — Multi-Agent Coordination
Last updated: 2026-02-15


## Before Starting Work

1. **Read `AGENTS.md`** — the entry point. Links to all other docs.
2. **Read `docs/STATUS.md`** — check what's done and what's in progress. If someone left work partially done, the details will be here.
3. **Read `docs/ARCHITECTURE.md`** — understand the module structure before touching code.
4. **Run `cargo test`** — confirm the codebase is green before you start.

## While Working

- Follow the conventions in `docs/DESIGN_PRINCIPLES.md`
- Keep files under ~200 lines
- Write tests for core logic
- Use `anyhow::Result` — no `unwrap()` on fallible ops, no `process::exit()` in library code

## After Completing Work

1. **Run `cargo test && cargo clippy --all-targets`** — leave the codebase green
2. **Update `docs/STATUS.md`:**
   - Move completed items from "In Progress" to "Completed" with details
   - Update line counts, test counts if they changed
   - If work is **partially done**, leave clear notes about what's left under the item
3. **Append to `docs/CHANGELOG.md`:**
   - Add a dated entry describing what was built
   - Include specifics: files changed, tests added, features implemented
4. **Update `docs/ARCHITECTURE.md`** if you added new modules or changed the structure

## Handoff Protocol

If you're stopping work mid-task:

1. **Commit or note all changes** — don't leave uncommitted work without explanation
2. **Update STATUS.md** with exactly where you stopped:
   - What's done
   - What's remaining
   - Any gotchas or blockers
   - Files you were working in
3. The next agent should be able to pick up seamlessly from STATUS.md alone

## Cross-Agent Messages

Use `docs/inbox/` for leaving messages to future agents:

```
docs/inbox/
  2026-02-14-note-about-watch-cmd.md
```

Format: `YYYY-MM-DD-<topic>.md`. Keep it brief. Delete after reading.

## Conflict Prevention

- **Check STATUS.md** before starting any "In Progress" item — another agent may already be working on it
- **Claim work** by adding your note to the STATUS.md item: `(Agent working on this as of YYYY-MM-DD)`
- **Don't refactor without reason.** If the tests pass and clippy is clean, the code is fine.

## What NOT to Do

- Don't restructure documentation without updating all cross-references
- Don't add dependencies to `Cargo.toml` without justification
- Don't delete tests (even if they seem redundant — they may catch regressions)
- Don't change the vault format without updating `README.md`, `ARCHITECTURE.md`, and all vault-related tests
