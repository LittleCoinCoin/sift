# Integrate Components and Update Action Controls

**Goal**: Integrate the new search and tree view into `ReceiptViewer.svelte` and update the actions.
**Pre-conditions**:
- [ ] `update_store.md` complete.
- [ ] `create_components.md` complete.
**Success Gates**:
- ✅ `ReceiptViewer.svelte` renders the new `FileSearchBar` and `ReceiptFileTree`.
- ✅ Tree/Flat view toggle button is present and functional.
- ✅ The "Process" button is moved to the bottom of the list and processes all `selectedPaths`.
- ✅ Right-click context menu provides access to actions (including Delete) on selected items.

## Step 1: Update ReceiptViewer UI (Agent: Implementor)
**Goal**: Replace flat list with new components and add toggle button.
**Implementation Logic**:
In `src/lib/ReceiptViewer.svelte`, remove the old `ul` file list and insert `FileSearchBar` and `ReceiptFileTree`. Add a header toggle to switch `viewMode` between "tree" and "flat". Pass the necessary bounds (receipt entries) and event handlers to the components.
**Deliverables**: `src/lib/ReceiptViewer.svelte` (Component updates: file list replaced)
**Consistency Checks**: `npm run check` (expected: PASS)
**Commit**: `feat(ui): integrate search and tree components into viewer`

## Step 2: Update Action Controls and Context Menu (Agent: Implementor)
**Goal**: Move the Process button and create a right-click context menu.
**Implementation Logic**:
Move the existing "Process" button to a dedicated area at the bottom of the file tree panel. Ensure its `onclick` handler iterates over `selectedPaths` in the store. Add a `ContextMenu.svelte` (if missing) and implement it for right-clicking selected files. Ensure there is a clear option to delete a receipt from the context menu.
**Deliverables**: `src/lib/ReceiptViewer.svelte` (Process button moved), `src/lib/ContextMenu.svelte` (Component: `ContextMenu` with process and delete actions)
**Consistency Checks**: `npm run check` (expected: PASS)
**Commit**: `feat(ui): update process action layout and add context menu`

## Step 3: End-to-end Integration Verification (Agent: Verifier)
**Goal**: Verify the UX flow, multi-selection, and process action triggers.
**Implementation Logic**:
Review the bindings and action handlers in `ReceiptViewer.svelte`. Verify that `selectedPaths` is iterated for processing, and that glob filtering is passed down properly. Check if delete action is present.
**Deliverables**: Final UX verification report.
**Consistency Checks**: `npm run check` (expected: PASS)
**Commit**: `test(ui): verify viewer integration and controls`
