# Sort Svelte Frontend

**Goal**: Enforce stable alphabetical ordering in the Svelte layer — in the receipt store, the tree/flat file view, and the extraction fields panel.
**Pre-conditions**:
- [ ] Branch `task/sort-svelte-frontend` created from the worktree branch
**Success Gates**:
- ✅ File tree (tree and flat view) renders files in alphabetical path order on every mount [behavioral]
- ✅ Subdirectories within an expanded tree node are alphabetical [behavioral]
- ✅ Extraction fields panel shows keys in alphabetical order for any processed receipt [behavioral]
- ✅ Field display order does not change while the user types in a field [behavioral]

---

## Step 1: Sort in the receipt store

**Goal**: Make `ReceiptStore.setFiles` the single source of truth for entry order so any ingestion path (scan, future reload) inherits alphabetical order automatically.

**Implementation Logic**:
In `src/lib/stores/receipts.svelte.ts`, `setFiles(newFiles)` currently assigns `this.files = newFiles` verbatim. Spread-copy `newFiles` and sort by `source_path` using `localeCompare` before assigning, so `this.files` is always sorted. Use the sorted array for the auto-selection fallback too. The spread avoids mutating the caller's array (Svelte 5 reactivity contract).

**Deliverables**:
- `src/lib/stores/receipts.svelte.ts` — `setFiles` (sorted assignment)

**Consistency Checks**: `pnpm typecheck` or `pnpm check` in the project root (expected: PASS)
**Commit**: `fix(store): sort receipt entries alphabetically in setFiles`

---

## Step 2: Sort in the file tree component

**Goal**: Guarantee alphabetical order in both view modes of `ReceiptFileTree` — the `$derived` filtered list (drives flat view and range-selection) and the `subDirs` in `buildRenderList` (the only tree-internal collection that loses order via Set de-duplication).

**Implementation Logic**:
In `src/lib/ReceiptFileTree.svelte`:

1. In the `filteredEntries` `$derived.by` block (lines ~78–94), add `.slice().sort((a, b) => a.source_path.localeCompare(b.source_path))` as the final step before `return`. The `.slice()` avoids mutating the `entries` prop reference (the `else` branch returns `entries` directly). This covers flat-view iteration, the `visiblePaths` derived used for shift-click range selection, and the input to `buildRenderList`.

2. In `buildRenderList`, inside `addDir`, add `.sort()` to the `subDirs` array literal (lines ~139–146) after the `...new Set(...)` spread. This is the only truly necessary sort inside `buildRenderList`: `Set` de-duplication loses order, and `filteredEntries` being pre-sorted already covers root files and direct children within directories (`.filter()` preserves relative order).

**Deliverables**:
- `src/lib/ReceiptFileTree.svelte` — `filteredEntries` (sorted return), `buildRenderList` → `addDir` → `subDirs` (`.sort()` added)

**Consistency Checks**: `pnpm typecheck` or `pnpm check` (expected: PASS)
**Commit**: `fix(tree): sort file entries and subdirectories alphabetically`

---

## Step 3: Sort fields in the viewer

**Goal**: Display extraction field keys in alphabetical order in the fields panel, without mutating store state.

**Implementation Logic**:
In `src/lib/ReceiptViewer.svelte`, the `{#each}` on line ~677 iterates `Object.entries(record.fields)` with no sort. Append `.sort(([a], [b]) => a.localeCompare(b))` to the `Object.entries(...)` call. This is purely presentational: the sort expression re-evaluates whenever `record.fields` changes (Svelte 5 fine-grained reactivity), but does not mutate store state, so the field order is stable while the user is typing — `updateField` modifies `entry.fields[key]` in-place which triggers a re-render of only the changed value, not a re-sort that would reorder the fields.

**Deliverables**:
- `src/lib/ReceiptViewer.svelte` — `{#each Object.entries(record.fields).sort(...)}` (line ~677)

**Consistency Checks**: `pnpm typecheck` or `pnpm check` (expected: PASS)
**Commit**: `fix(viewer): display extraction fields in alphabetical key order`
