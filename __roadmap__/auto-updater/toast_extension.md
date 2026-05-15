# Extend Toast System for Action Buttons

**Goal**: Add optional `action` (label + callback) and `persistent` (no auto-dismiss) fields to the toast system, plus two new helper exports needed by the updater store.
**Pre-conditions**:
- [ ] Branch created from `main`
**Success Gates**:
- ✅ `pnpm check` passes with no TypeScript errors [run]
- ✅ `Toast` interface in `log.ts` exposes `action?` and `persistent?` fields [static]
- ✅ `showToastReturningId` and `updateToastMessage` are exported from `log.ts` [static]
- ✅ `Toast.svelte` renders an action button (in place of the copy button) when `toast.action` is present [static]
**References**: [R01 Plan](../../../.claude/plans/assuming-the-current-project-enchanted-meteor.md) — toast extension design (§ Work breakdown D)

---

## Step 1: Extend the Toast type and store logic

**Goal**: Add `action?`, `persistent?`, helper exports, and update `showToast` — all in `src/lib/stores/log.ts`.

**Implementation Logic**: Four changes to `src/lib/stores/log.ts`:
1. Add `action?: { label: string; onClick: () => void }` and `persistent?: boolean` to the `Toast` interface (which extends `LogEvent`).
2. Update `showToast` to accept an optional third argument `opts?: { action?: ...; persistent?: boolean }` and pass those fields onto the created `Toast` object. The auto-dismiss `setTimeout` must be skipped when `opts?.persistent` is true or when `opts?.action` is defined (since the user must act to dismiss).
3. Add `showToastReturningId(level, message, opts?)` — identical to `showToast` but returns the numeric toast `id`. The updater store needs this to target a specific toast for later mutation or dismissal.
4. Add `updateToastMessage(id: number, message: string)` — mutates the `message` field of an existing toast in-place via `toasts.update`. Used to show download progress percentage without spawning a new toast.

**Deliverables**: `src/lib/stores/log.ts` — updated `Toast` interface; updated `showToast` signature; new exports `showToastReturningId` and `updateToastMessage`

**Consistency Checks**: `pnpm check` (expected: PASS)

**Commit**: `feat(toast): add action button and persistent flag to Toast type`

---

## Step 2: Render action button in Toast component

**Goal**: Display the action button in the toast UI, replacing the copy button when an action is present.

**Implementation Logic**: In `src/lib/Toast.svelte`, modify the per-toast button row so that: when `toast.action` is defined, render a `<button class="toast-action">` displaying `toast.action.label`, with an `onclick` handler that calls `toast.action.onClick()`. When `toast.action` is absent, render the existing copy button as before. The dismiss (×) button always renders in both cases. Add a `.toast-action` CSS rule: inherit `color`, no `background`, a subtle underline or faint border to signal interactivity, `flex-shrink: 0`, `font-family: var(--font-mono)`, `cursor: pointer`.

**Deliverables**: `src/lib/Toast.svelte` — conditional `toast-action` button replacing copy button; `.toast-action` CSS rule

**Consistency Checks**: `pnpm check` (expected: PASS)

**Commit**: `feat(toast): render action button in toast component`
