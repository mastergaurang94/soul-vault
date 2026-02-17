# Cloud Import Across All Providers
Last updated: 2026-02-17

## Status

- `in_progress`

## Goal

- Implement real cloud conversation import for all supported providers (Claude, ChatGPT, Gemini), using provider-scoped OAuth/session ownership and async progress reporting in TUI and CLI.

## Acceptance Criteria

- [x] `soul import --cloud --provider chatgpt|gemini` fetches real conversations from provider APIs and imports them into vault; `claude` returns explicit export-based fallback guidance.
- [x] No implicit provider default for cloud import flows in interactive UX: user explicitly chooses provider.
- [x] OAuth in init/settings is provider-scoped and clearly linked to that provider's cloud import capability.
- [x] Long-running cloud imports run asynchronously with visible progress and final summary in TUI.
- [x] Source tracking prevents duplicate re-imports of unchanged cloud conversations.
- [x] Errors are actionable and provider-specific (auth revoked, rate limited, quota, schema mismatch).
- [ ] Test coverage includes pagination, retries, token refresh, and partial-failure behavior.

## Scope Boundaries

- In scope:
  - Cloud fetch + normalization + import for Claude, ChatGPT, Gemini.
  - Async job orchestration and progress telemetry for CLI/TUI.
  - Provider-scoped auth/connect status and import readiness messaging.
- Out of scope:
  - New providers beyond the 3 current cloud targets.
  - Full account/device sync service beyond local vault import.

## Pre-conditions

- Required context/docs reviewed:
  - [x] `AGENTS.md`
  - [x] `docs/STATUS.md`
  - [x] `docs/ADAPTERS_SPEC.md`
  - [x] `docs/ARCHITECTURE.md`
  - [x] `docs/TUI_SPEC.md`
- Environment prerequisites met:
  - [x] Build/test commands pass on current baseline before implementation.

## Architecture Decisions (up front)

- Provider choice:
  - Cloud import never silently assumes Claude in interactive flows.
  - CLI may keep a documented default only for backward compatibility, but interactive prompts must require explicit provider selection.
- OAuth ownership:
  - OAuth credentials are stored and evaluated per provider.
  - Settings and status surfaces must show provider-scoped readiness, not global "connected".
- Async behavior:
  - Cloud imports execute in background tasks in TUI with cancellable/progressive updates.
  - CLI remains streaming/progressive (single process) but uses the same job/progress model internally.

## Steps

- [x] Step 1: Cloud domain model + provider client trait
  - Define `CloudProviderClient` trait and normalized cloud conversation/message model.
  - Add shared pagination and retry wrapper APIs.

- [x] Step 2: Unified auth/token layer for cloud clients
  - Reuse existing OAuth store and refresh paths.
  - Add provider-specific token validation and refresh decision helpers.

- [x] Step 3: Claude cloud path decision
  - Verified no documented Anthropic cloud history/list endpoint in public API docs.
  - Wire explicit `--provider claude` fallback guidance to export/local import path.

- [x] Step 4: ChatGPT cloud client implementation
  - Implement list/fetch/paginate/normalize.
  - Wire into provider switch + import pipeline.

- [x] Step 5: Gemini cloud client implementation
  - Implement list/fetch/paginate/normalize.
  - Wire into provider switch + import pipeline.

- [x] Step 6: Source tracking for cloud IDs and incremental sync
  - Extend source tracking to include provider conversation IDs + content/version/hash markers.
  - Skip unchanged cloud conversations by default.

- [x] Step 7: Async import orchestration + progress events
  - Introduce import job state (`queued`, `fetching`, `normalizing`, `processing`, `writing`, `done`, `failed`).
  - Emit progress counters and current conversation/provider context.

- [x] Step 8: TUI integration
  - Add provider picker in import flow (explicit selection).
  - Add cloud import progress view and completion summary.
  - Support user cancellation with safe partial commit behavior.

- [x] Step 9: Init/settings UX alignment
  - Ensure provider OAuth selection maps directly to provider cloud import readiness.
  - Remove or revise copy that implies generic default provider behavior.

- [x] Step 10: Reliability hardening
  - Rate-limit handling, retries with backoff, partial failure aggregation.
  - Resume-friendly behavior where feasible.

- [ ] Step 11: Verification + docs
  - Add/expand unit + integration tests for all providers.
  - Update `docs/STATUS.md`, `docs/CHANGELOG.md`, and usage docs.

## Decision Log

- `2026-02-17`: Do not default interactive cloud import to Claude; require explicit provider selection. Rationale: avoids wrong-account assumptions and aligns with provider-scoped OAuth setup.
- `2026-02-17`: Cloud imports should be async with progress/cancellation in TUI. Rationale: large accounts can take significant time; blocking UX is poor.
- `2026-02-17`: OAuth completion alone is insufficient; readiness must include provider-specific fetch capability + token validity checks.
- `2026-02-17`: Cloud clients now use provider-specific endpoints with env-overridable base URLs and shared retry/error mapping. Rationale: allows shipping real network path while preserving flexibility for endpoint drift.
- `2026-02-17`: Settings/init/login OAuth UX now reports provider-scoped configuration readiness rather than "coming soon" for ChatGPT/Gemini. Rationale: align UI readiness with provider-scoped cloud import behavior.

## Risks And Blockers

- Risk: Provider API schema divergence and frequent response shape changes.
  - Impact: Parser/import breakage per provider.
  - Mitigation: Provider-specific adapters + schema tests + robust fallback parsing.

- Risk: Rate limits and quota errors on large history imports.
  - Impact: Long imports fail midway.
  - Mitigation: Exponential backoff, checkpointed progress, partial-success summaries.

- Risk: Token refresh edge cases and revoked sessions.
  - Impact: Confusing auth failures.
  - Mitigation: Clear provider-scoped remediation messages and reconnect prompts.

- Blocker: Missing stable API endpoints/permissions for one or more providers.
  - Owner: Implementation owner.
  - Next action: Feature-gate provider path, ship working providers first, preserve explicit "coming soon" copy for blocked provider.

## Verification Criteria

- [x] `cargo build --release`
- [x] `cargo test`
- [x] `cargo clippy --all-targets -- -D warnings`
- [ ] Provider integration tests with mocked APIs for list/fetch/pagination/retry/refresh.
  - Current blocker in this environment: sandbox prevents binding mock HTTP ports (`Operation not permitted`), so unit coverage was added for parsing/retry/error/token-path logic instead.
- [ ] Manual TUI validation:
  - explicit provider selection
  - visible progress updates
  - cancellation path
  - clear final summary
- [x] Docs updated:
  - `docs/STATUS.md`
  - `docs/CHANGELOG.md`
  - relevant command usage docs.

## Execution Order

1. Foundation + auth abstraction.
2. Claude end-to-end path.
3. ChatGPT end-to-end path.
4. Gemini end-to-end path.
5. Async UX hardening + docs.

## Notes

- This plan intentionally aligns init/settings OAuth choices with cloud import execution semantics to avoid mismatched states.
- Any provider-specific block should not stop shipping other providers; use explicit capability reporting.
