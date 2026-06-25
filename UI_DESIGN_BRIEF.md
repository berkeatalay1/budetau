# Budget Planner — UI Design Brief

> **Hand this to a designer.** It contains everything needed to design the UI: product context, brand tokens, every screen with its states, a component inventory, interaction patterns, and the deliverables requested. Where a decision is still open, it's flagged **[CONFIRM]**.

---

## 1. Product in one paragraph

A **local-first personal budget planner** — a native desktop app (macOS first) where all data lives in a single private SQLite file the user can keep in a cloud folder. No accounts, no servers. The emotional goal: **calm, fast, and trustworthy.** People open it to log a coffee in two seconds or to glance at "am I over budget this month?" — not to wrestle a spreadsheet. Aesthetic: clean, modern, card-based, generous whitespace. Think *Linear / Things 3 / Copilot Money*, not *Excel*.

## 2. Design principles

1. **Speed over chrome.** The most common action (log a transaction) must be reachable in one keystroke (`Cmd/Ctrl+K`) and done in under 5 seconds.
2. **Cards, not grids.** Display data as clean cards with color and hierarchy, never dense spreadsheet rows.
3. **Glanceable status.** A user should understand their month in 2 seconds from the dashboard — green = fine, amber = watch, red = over.
4. **Privacy as a feeling.** The UI should quietly reassure (e.g. Settings shows exactly where the local file lives). Nothing feels "cloud."
5. **Quiet by default, loud when it matters.** Alerts only appear when a budget is genuinely at risk.

## 3. Brand & visual tokens (already fixed in code — design around these)

**Color palette** (CSS variables already exist in the app):

| Token | Light | Role |
|---|---|---|
| `--primary` | `#1f2937` (gray-800) | Primary text, headings |
| `--secondary` | `#4b5563` (gray-600) | Secondary text |
| `--bg` | `#f9fafb` (gray-50) | App background |
| `--card` | `#ffffff` | Card surfaces |
| `--border` | `#e5e7eb` (gray-200) | Dividers, card borders |
| `--accent` | `#2563eb` (blue-600) | Primary actions, links, focus |
| `--green` | `#059669` | Income, positive, "under budget" |
| `--amber` | `#d97706` | Warning, "near limit" |
| `--red` | `#dc2626` | Expense over, "over budget", destructive |

- **Category colors** are user/data-driven (each category has a `color_code` hex, e.g. Dining `#E57373`, Groceries `#81C784`). Design components to accept an arbitrary category color as an accent dot/bar.
- **Dark mode is required** — propose a full dark token set (don't just invert). Keep the same semantic roles.
- **Typography:** system font stack (`-apple-system, "Segoe UI", Roboto, …`). Propose a clear type scale (display / h1 / h2 / body / caption / mono-for-amounts). Amounts may use tabular/mono figures so they align.
- **Shape:** rounded corners (cards ~12px, controls ~8px, pills/badges 999px). Soft shadows, not heavy borders.

## 4. Information architecture

Persistent **left sidebar** + main content area. Three routes already exist:

- **Dashboard** (`/dashboard`, default) — the home glance.
- **Transactions** (`/transactions`) — the full ledger.
- **Settings** (`/settings`) — local DB path, theme, backup.

A **global Quick-Add modal** overlays any screen (FAB bottom-right + `Cmd/Ctrl+K`).

## 5. Screens to design

For **each** screen, deliver light + dark, and the empty / loading / populated states.

### 5.1 App shell
- Fixed left sidebar: app name/logo lockup, the 3 nav items (icon + label, active state), and a footer slot (e.g. current month, or profile name "Personal").
- Main area scrolls independently. Define behavior at narrow widths (sidebar collapses to icons or a top bar — **[CONFIRM]** minimum window width target).

### 5.2 Dashboard — *the headline screen*
Compose these blocks (propose the layout/hierarchy):
1. **Hero summary** — Total Spend vs Total Income for the current month, plus net. Big, glanceable numbers; subtle month selector.
2. **Alert banners** — appear only when a category is **near (≥90%)** or **over** its limit. e.g. *"Dining Out is at 95% of budget."* Two severities: warning (amber) and over (red). Dismissible? **[CONFIRM]**.
3. **Budget cards grid** — responsive grid, one card per active category. Each card: category color, name, **spent vs remaining progress bar**, amount spent / limit. Progress bar color shifts green → amber → red as it fills.
4. **Empty state** — first-run user with no transactions/budgets yet: a friendly prompt to add their first transaction or set a budget.

### 5.3 Transactions — *the ledger*
- **Transaction list** of `TransactionCard`s: category color dot/icon, note/category name, account, **amount** (income green, expense default/red), date. Grouped by day with date headers.
- **Search** (by note/category) + **filters**: date range, account, category, type (income/expense/transfer). Design the filter UI (chips? a filter bar? a side drawer?).
- **Row actions:** edit and delete per transaction (hover affordance on desktop). Deleting needs a confirm.
- **Empty / no-results** states (no transactions vs filters matched nothing — distinct).

### 5.4 Quick-Add modal — *the speed feature*
- Invoked by FAB or `Cmd/Ctrl+K`; centered modal, keyboard-first.
- **Default (streamlined):** three fields only — **Amount**, **Category** (autocomplete from existing categories, shows color), **Date** (defaults to today).
- **"Add details" toggle** reveals: Note, Account selector, Type (expense/income/transfer), and — when type=transfer — a destination account.
- Show focus order, the autocomplete dropdown, validation (e.g. amount required), and the success confirmation (toast? inline?).

### 5.5 Settings
- **Database card:** show the resolved local file path (already wired) with a "reveal in Finder" affordance — this is the privacy reassurance moment.
- **Appearance:** light / dark / system theme toggle.
- **Backup & restore:** export a timestamped copy; restore from a file (with a clear warning before overwrite).

## 6. Component inventory (design as a small system)

Sidebar / NavItem · HeroSummary · AlertBanner (warning/over) · BudgetCard (+ ProgressBar) · TransactionCard · DayGroupHeader · FilterBar/Chips · SearchInput · QuickAddModal · CategoryAutocomplete (with color swatch) · AccountSelect · AmountInput (currency-formatted) · FAB · Toast/Notification · Button (primary/secondary/ghost/destructive) · EmptyState · Toggle/Switch · ConfirmDialog.

For each component show its **states**: default, hover, focus (keyboard ring in `--accent`), active, disabled, loading, error.

## 7. Data & formatting rules (important for realistic mockups)

- **Money is stored as integer cents.** Always display formatted currency (e.g. `1234` → `$12.34`). Use the account's currency symbol; assume **USD `$`** for mockups unless told otherwise. Right-align amounts; consider tabular figures.
- **Transaction types:** `expense` (most common), `income`, `transfer` (moves between two accounts). Color/sign accordingly (expense negative, income positive/green).
- **Categories are flat** (no nesting) and each has a color — lean on that color for visual identity.
- Use **realistic sample data** in mockups: categories like Dining Out, Groceries, Rent, Transport, Salary; a mix of recent transactions; one budget near its limit and one over, so the alert/progress states are visible.

## 8. Interaction & motion

- Keyboard-first: `Cmd/Ctrl+K` opens Quick-Add; `Esc` closes modals; arrow keys navigate the category autocomplete.
- Subtle, fast motion (~150–200ms): modal scale/fade, progress-bar fill, toast slide-in. Nothing bouncy or slow.
- Optimistic UI: a newly added transaction appears instantly.

## 9. Responsive & platform

- **Primary target:** desktop window (macOS), resizable. Design at a comfortable default (~1200px) and a narrow (~720px) breakpoint.
- Mobile is **not** in current scope, but keep components flexible (don't hard-code widths) — **[CONFIRM]** if a tablet layout is wanted.

## 10. Accessibility

- WCAG AA contrast in **both** themes (watch amber/green on white).
- Don't rely on color alone for budget status — pair with icon/label and the numeric %.
- Visible keyboard focus everywhere; full keyboard operability of the Quick-Add flow.
- Respect `prefers-reduced-motion`.

## 11. Deliverables requested

1. A **style tile / foundations** page: full light + dark token sets, type scale, spacing, the shared components with states.
2. **High-fidelity mockups** for: App shell, Dashboard (populated + empty), Transactions (populated + no-results), Quick-Add modal (collapsed + expanded), Settings — each in **light and dark**.
3. The **budget status spectrum** shown explicitly: a card/progress bar at under-budget (green), near-limit (amber), and over (red), plus the matching alert banners.
4. Redlines/notes on spacing and the responsive behavior at the narrow breakpoint.

## 12. Open questions for the product owner [CONFIRM]

- Minimum supported window width / is a tablet layout wanted?
- Are alert banners dismissible, or persistent until resolved?
- Quick-Add success feedback: toast vs inline vs auto-close?
- Default currency for mockups (assuming USD `$`).
- Any logo / wordmark, or should the designer propose a simple lockup for "Budetau"?
