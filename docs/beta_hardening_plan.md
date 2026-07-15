# Budetau — Beta / Production Hardening Plan

**Purpose:** work breakdown for the fixes that stand between the current build and an honest beta `.dmg` (and, later, a 1.0). Derived from the Fable production-readiness review. Branch `phase-1-local-infra`; baseline HEAD at time of writing ≈ `1663c2a` (new icon) + the in-flight Settings overhaul.

**How to use:** each task has an ID, the Fable finding it maps to, priority, the wave it belongs to (for parallelization), the files it touches, and acceptance criteria. Follow the same workflow used all session: **Wave 0 backend done by one owner (shared `db.rs`/`commands.rs`/`lib.rs` — do NOT parallelize these)** → commit checkpoint → **Wave 1 frontend/config in parallel on disjoint files** → integration + rebuild `.dmg`. Each T can be a Sonnet agent. Verify every change with the mock-invoke pattern; keep the full gate green (build → lint → build → `cargo test` → clippy → vitest).

---

## Beta gate (minimum to hand testers a `.dmg`)
**Must fix:** H1, H2, H3, H4, H5, H6 · **Must verify:** H7 · **Cheap + high-value, include:** H8 (reject newer schema), H14 (CSP).
Everything else is 1.0 / polish.

---

## Task table

| ID | Fable | Title | Priority | Wave | Files | Depends |
|----|-------|-------|----------|------|-------|---------|
| **H1** | B2 | Atomic migrations + pre-migrate backup | 🔴 beta | 0 | `db.rs` | — |
| **H2** | B3 | Recurring scheduler: advance `next_due` per-occurrence in one txn | 🔴 beta | 0 | `commands.rs` | — |
| **H3** | B1 | Graceful startup-failure dialog (no silent crash) | 🔴 beta | 0 | `lib.rs`, `db.rs` | H1 |
| **H4** | B4 | Transaction edit/delete error handling + global error net | 🔴 beta | 1 | `TransactionsView.vue`, `main.ts` | — |
| **H5** | B5 | Local logging (`tauri-plugin-log`) | 🔴 beta | 0 | `lib.rs`, `Cargo.toml`, `commands.rs`, capabilities | — |
| **H6** | B6 | DB location: `app_data_dir` + handle denied access | 🔴 beta | 0 | `db.rs` | H1,H3 |
| **H7** | B7 | Verify + fix deep-path reload in packaged build | 🔴 beta | 1 | `router/index.ts`, `SettingsView.vue` | — |
| **H8** | S1 | Refuse to open `user_version > CURRENT` | 🟠 1.0 (incl. in beta) | 0 | `db.rs` | H1 |
| **H9** | S2 | Atomic restore (temp+rename; reopen original on failure) | 🟠 1.0 | 0 | `commands.rs` | — |
| **H10** | S7 | Mutex poisoning recovery (`parking_lot` or `into_inner`) | 🟠 1.0 | 0 | `commands.rs`, `lib.rs`, `Cargo.toml` | — |
| **H11** | S4 | `set_assigned` validates category type (+ parent-ness) server-side | 🟠 1.0 | 0 | `commands.rs` | — |
| **H12** | S5 | FK-guard account/category deletes against `recurring_rules` | 🟠 1.0 | 0 | `commands.rs` | — |
| **H13** | S3 | Multi-currency: single-currency-per-profile for v1 (drop/scope picker) | 🟠 1.0 | 1 | `AccountFormModal.vue`, `format.ts`, budget math | decision |
| **H14** | S13 | Set a strict CSP (`default-src 'self'`) | 🟠 1.0 (incl. in beta) | 2 | `tauri.conf.json` | — |
| **H15** | S6 | Recurring posts on a daily/focus trigger, not just launch | 🟠 1.0 | 1 | `App.vue`/store + `postDueRecurring` | — |
| **H16** | S11 | Add `transactions` indexes (date, category_id, account_id) | 🟠 1.0 | 0 | `db.rs` (migration v7) | H1 |
| **H17** | S12 | Bump `minimumSystemVersion` to 11.0 (aarch64 reality) | 🟠 1.0 | 2 | `tauri.conf.json` | — |
| **H18** | S8 | LICENSE + privacy statement + OSS attribution (About card) | 🟠 1.0 | 2 | `LICENSE`, About card, docs | — |
| **H19** | S9 | CI: run build/lint/vue-tsc/vitest + cargo test/clippy on push | 🟠 1.0 | 2 | `.github/workflows/` | — |
| **H20** | S10 | Update story (updater or in-app "check Releases" + changelog) | 🟠 1.0 | 2 | config / About card | decision |
| **P1** | polish | `tauri-plugin-window-state` (remember size/pos) | 🟢 | 2 | `lib.rs`, `Cargo.toml` | — |
| **P2** | polish | Native app menu + ⌘1–5 view shortcuts, ⌘, → Settings | 🟢 | 1 | `lib.rs` and/or `App.vue` | — |
| **P3** | polish | Automatic periodic (daily / on-quit) backup into app-data | 🟢 | 0 | `commands.rs`, `lib.rs` | — |
| **P4** | polish | DMG background + Applications symlink layout | 🟢 | 2 | `tauri.conf.json` bundle.macOS.dmg | — |
| **P5** | polish | Kill theme-flash-on-launch (apply theme before mount) | 🟢 | 1 | `index.html`/`main.ts`/`theme.ts` | — |

---

## Task detail (what / why / acceptance)

### 🔴 Beta blockers

**H1 — Atomic migrations + pre-migrate backup** *(Fable B2)*
- **What:** wrap each `migrate_vN` + its `PRAGMA user_version = N` in a single transaction so a failure rolls back cleanly and never leaves the DB half-migrated. Before migrating any DB with `user_version > 0`, file-copy it to a `pre-migrate-<version>-<stamp>.db` safety copy (reuse the pre-restore prune helper pattern).
- **Why:** `migrate_v4`'s `ALTER` + separate `UPDATE` can strand the schema (col added, version not bumped) → "duplicate column" crash loop forever. Beta testers' DBs get migrated by every future release.
- **Accept:** unit test: a migration whose second statement fails leaves `user_version` and schema unchanged (rolled back); a pre-migrate backup file is written; existing 46 tests still pass.

**H2 — Recurring scheduler idempotency** *(Fable B3)*
- **What:** in `post_due_recurring`, advance and persist `next_due` inside the same transaction as each posted occurrence (or wrap the per-rule loop so a crash can't leave posted rows with a stale `next_due`).
- **Why:** today `next_due` is written only after the whole catch-up loop; a crash mid-loop reposts occurrences next launch → duplicate paychecks/rent.
- **Accept:** test simulating a failure between post N and post N+1 leaves no rule whose `next_due` is behind its already-posted occurrences; idempotency test still green.

**H3 — Graceful startup failure** *(Fable B1)*
- **What:** in `lib.rs setup()`, catch DB-open/init/migrate errors and show a native dialog (dialog plugin already present) with the DB path + "restore from a backup" guidance, instead of `.expect(...)` → dock-bounce-and-die.
- **Why:** the single worst failure mode for a data-safety app; pairs with H1/H6.
- **Accept:** simulate a corrupt DB → app shows an explanatory dialog rather than crashing silently. (Verify in a real build.)

**H4 — Transaction error handling + global net** *(Fable B4)*
- **What:** add try/catch + toast to `TransactionsView` `handleSave`/`handleConfirmDelete`; add a global `app.config.errorHandler` and a `window.addEventListener('unhandledrejection', …)` in `main.ts` that surface via the toast store.
- **Why:** edit/delete failures currently vanish; user believes a change saved when it didn't.
- **Accept:** a rejected update/delete shows an error toast and keeps the row; global handler catches an otherwise-unhandled rejection. Mock-invoke verify.

**H5 — Local logging** *(Fable B5)*
- **What:** add `tauri-plugin-log` writing to the app-log dir; log migrations run, recurring posts, backup/restore, and command errors. Local-only (privacy-consistent).
- **Why:** `println!` goes nowhere in a bundled app; beta reports are otherwise undebuggable.
- **Accept:** log file appears in the app-log dir with entries after launch/actions; no PII beyond what's already local.

**H6 — DB location** *(Fable B6)*
- **What:** default the DB to `app_data_dir` (no TCC prompt, no iCloud eviction, name matches product) instead of `~/Documents/BudgetPlanner`; handle a denied-access/create failure via H3's dialog. Keep the Settings "File location / Copy path" card pointing at the real path. Decide on a one-time migration of an existing Documents DB (copy-forward) vs. leaving legacy users where they are.
- **Why:** Documents triggers TCC on unsigned apps (deny → crash loop), iCloud can evict the file, and the folder name confuses testers.
- **Accept:** fresh install creates the DB under app-data; existing DB handled per the chosen migration; denied access is explained, not crashed.

**H7 — Packaged deep-path reload** *(Fable B7)*
- **What:** confirm whether `createWebHistory` + `window.location.reload()` at `/settings` white-screens under the `tauri://` protocol in the **packaged** build (the restore flow does exactly this). If it reproduces, switch to `createWebHashHistory` or reload to `/`.
- **Why:** if it reproduces, *every* successful restore ends on a blank window.
- **Accept:** in a real `.dmg`, run a restore → app returns to a working screen, not blank.

### 🟠 Should-fix before 1.0 (H8–H20)
Concise intents (see Fable §2 for full reasoning):
- **H8** reject `user_version > CURRENT_SCHEMA_VERSION` on open (downgrade / newer-backup safety). *Cheap — include in beta.*
- **H9** restore = copy to temp + atomic rename; on any failure reopen the ORIGINAL DB into the guard (never strand an in-memory placeholder).
- **H10** replace `std::sync::Mutex` with `parking_lot::Mutex` (or recover poisoned locks) so one panic doesn't brick every command.
- **H11** `set_assigned` rejects non-expense categories (and optionally parents) server-side — currently assigning to income silently drains RTA with no visible row.
- **H12** delete guards for accounts/categories referenced by `recurring_rules` (friendly message, not raw FK error).
- **H13** decide multi-currency: for v1, one currency per profile (drop the account currency picker or exclude non-default from budget math) + handle zero-decimal currencies (JPY) or defer them. Current state = silently wrong budgets for non-USD accounts.
- **H14** set `app.security.csp` to `default-src 'self'` (+ `style-src 'self' 'unsafe-inline'` if needed for Tailwind/inline styles; ApexCharts is bundled, no remote). *Cheap — include in beta.*
- **H15** call `postDueRecurring()` on a daily timer / window-focus (already wired in `api.ts`, nothing calls it) so long-open Macs still post.
- **H16** migration v7: `CREATE INDEX` on `transactions(date)`, `(category_id)`, `(account_id)` — cheap now, saves pain at scale.
- **H17** `minimumSystemVersion` → "11.0".
- **H18** add `LICENSE`, a short privacy note (reinforces the local-first pitch), and OSS attribution in the About card.
- **H19** GitHub Actions: frontend (build/lint/vue-tsc/vitest) + backend (cargo test/clippy) on push/PR.
- **H20** decide the update path (`tauri-plugin-updater` + signed releases, or an in-app "you're on vX — check Releases" link) + a CHANGELOG.

### 🟢 Polish (P1–P5)
Window-state persistence, native menu + shortcuts, automatic periodic backup, DMG presentation, theme-flash fix. Do opportunistically.

---

## Sequencing notes
- **Wave 0 is backend-heavy and mostly touches `db.rs` / `commands.rs` / `lib.rs`** (H1, H2, H3, H5, H6, H8, H9, H10, H11, H12, H16, P3). These collide → **one owner, sequential**, commit as a checkpoint. Natural sub-order: H1 → H8 → H16 (migration/schema) ; H2, H9, H10, H11, H12 (commands) ; then H3 + H6 + H5 (lib.rs/startup). 
- **Wave 1 is frontend/disjoint and parallelizable:** H4 (`TransactionsView`+`main.ts`), H7 (`router`), H13 (`AccountFormModal`/`format`), H15 (`App.vue`), P2/P5. One Sonnet agent per file cluster.
- **Wave 2 is config/meta:** H14, H17, H18, H19, H20, P4 — small, low-risk, can batch.
- **Integration:** full gate + a **real packaged-build smoke test** (esp. H3, H6, H7 only manifest in the bundle) → rebuild the `.dmg`.

## Also queued (from prior sessions, unrelated to Fable)
- **CSV bank import** — next committed roadmap feature (migration v8 after H16's v7), `import_batches` table, parser + preview + dedupe.
- Removing the dead legacy `budgets` backend once nothing needs it (kept for the v5 backfill).
