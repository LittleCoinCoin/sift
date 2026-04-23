# Update Receipt Store for Multi-Selection

**Goal**: Update the receipt store to handle a set of selected receipts instead of a single selection.
**Pre-conditions**:
- [ ] Requirements understood and store codebase read.
**Success Gates**:
- ✅ `src/lib/stores/receipts.svelte.ts` uses an array or Set for `selectedPaths`.
- ✅ Store exposes methods to add, remove, toggle, and clear multiple selection paths.
**References**: [R01 §ReceiptViewer](src/lib/ReceiptViewer.svelte) — existing single-selection references

## Step 1: Update Store Implementation (Agent: Implementor)
**Goal**: Change the `selectedPath` store variable to `selectedPaths` collection.
**Implementation Logic**:
Change `selectedPath` from string to `selectedPaths` as a collection (e.g. Set or Array). Add methods `addSelection(path)`, `removeSelection(path)`, `toggleSelection(path)`, and `clearSelection()`. Update any logic in `src/lib/stores/receipts.svelte.ts` that relies on the single selection string. Fix minor compiler errors in `src/lib/ReceiptViewer.svelte` to use the new collection format minimally so the build passes.
**Deliverables**: `src/lib/stores/receipts.svelte.ts` (store methods: `selectedPaths`, `addSelection`, `removeSelection`, `toggleSelection`, `clearSelection`)
**Consistency Checks**: `npm run check` (expected: PASS)
**Commit**: `feat(store): convert selection to handle multiple paths`

## Step 2: Verify Store Changes (Agent: Verifier)
**Goal**: Verify that store changes compile and the new methods exist.
**Implementation Logic**:
Read the updated store file and check that the exported methods correctly mutate the `selectedPaths` collection. Ensure no type errors exist.
**Deliverables**: Verification report.
**Consistency Checks**: `npm run check` (expected: PASS)
**Commit**: `test(store): verify multi-selection store update`
