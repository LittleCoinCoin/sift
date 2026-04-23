# Implement Search/Filter and View Components

**Goal**: Create reusable components for the search/filter bar (glob pattern only) and tree/flat file view.
**Pre-conditions**:
- [ ] Reference components in `tmp/` are analyzed.
**Success Gates**:
- ✅ `src/lib/FileSearchBar.svelte` correctly implements glob text and status filtering.
- ✅ `src/lib/ReceiptFileTree.svelte` displays `ReceiptEntry` nodes using `ReceiptFileNode.svelte` and `ReceiptDirectoryNode.svelte`.
- ✅ Native multi-selection patterns (Ctrl/Cmd-click, Shift-click, Cmd-A) are implemented inside the tree component.
**References**: [R01 §tmp/CorpusFileTree](tmp/CorpusFileTree.svelte) — reference for file tree component structure

## Step 1: Create Search Bar (Agent: Implementor)
**Goal**: Build `FileSearchBar.svelte` with glob pattern and status filtering.
**Implementation Logic**:
Use `tmp/FileSearchBar.svelte` as a starting point. Keep it simple: only support glob patterns (e.g., `*`, `?`) and status filtering for ReceiptEntry statuses. No regex. Ensure it exposes an `onfilter` event.
**Deliverables**: `src/lib/FileSearchBar.svelte` (Component: `FileSearchBar`)
**Consistency Checks**: `npm run check` (expected: PASS)
**Commit**: `feat(ui): implement FileSearchBar with glob support`

## Step 2: Create Tree and Node Components (Agent: Implementor)
**Goal**: Build `ReceiptFileTree`, `ReceiptDirectoryNode`, and `ReceiptFileNode`.
**Implementation Logic**:
Adapt `tmp/CorpusFileTree.svelte`, `tmp/CorpusDirectoryNode.svelte`, and `tmp/CorpusFileNode.svelte` into `src/lib/`. They must display `ReceiptEntry` items from `receipts` store. Implement `viewMode` toggle (tree vs flat). In flat mode, prefix receipt filenames with their directory path. Implement standard native selection interactions (Ctrl/Cmd+click, Shift+click, select all, parent directory selection selects all children).
**Deliverables**: `src/lib/ReceiptFileTree.svelte` (Component: `ReceiptFileTree`), `src/lib/ReceiptDirectoryNode.svelte` (Component: `ReceiptDirectoryNode`), `src/lib/ReceiptFileNode.svelte` (Component: `ReceiptFileNode`)
**Consistency Checks**: `npm run check` (expected: PASS)
**Commit**: `feat(ui): implement file tree components with multi-select`

## Step 3: Verify View Components (Agent: Verifier)
**Goal**: Ensure all components exist, compile properly, and follow the UI requirements.
**Implementation Logic**:
Verify that the `ReceiptFileTree.svelte` contains proper Svelte 5 logic (`$state`, `$derived`), bindings for `selectedPaths`, and correctly interprets glob searches.
**Deliverables**: Verification report.
**Consistency Checks**: `npm run check` (expected: PASS)
**Commit**: `test(ui): verify file tree component structure`
