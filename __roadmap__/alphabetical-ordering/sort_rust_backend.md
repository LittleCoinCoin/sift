# Sort Rust Backend

**Goal**: Sort the `Vec<ReceiptEntry>` returned by `scan_all_receipt_dirs` alphabetically by `source_path` so the backend always emits a deterministic, ordered list.
**Pre-conditions**:
- [ ] Branch `task/sort-rust-backend` created from the worktree branch
**Success Gates**:
- ✅ `cargo test -p sift` passes, including the new `sort_order_is_alphabetical_by_source_path` test [run]
- ✅ The last line of `scan_all_receipt_dirs` is a sorted collect, not a bare `into_values().collect()` [static]

---

## Step 1: Sort and test

**Goal**: Replace the bare `into_values().collect()` with a sort-then-collect, and add a unit test that verifies the predicate.

**Implementation Logic**:
`scan_all_receipt_dirs` (`src-tauri/src/scan.rs`, line 105) currently returns `index.into_values().collect()` where `index` is a `HashMap` — insertion order is undefined. Collect into a `Vec`, call `.sort_by(|a, b| a.source_path.cmp(&b.source_path))` (lexicographic byte order, correct for file-path strings), then return. Add a `#[test]` in the existing `#[cfg(test)]` block that builds a small hand-crafted `Vec<ReceiptEntry>`, applies the same sort predicate, and asserts the resulting order — this validates the predicate without needing a Tauri `AppHandle`.

**Deliverables**:
- `src-tauri/src/scan.rs` — `scan_all_receipt_dirs` (sorted return), `sort_order_is_alphabetical_by_source_path` (new test)

**Consistency Checks**: `cargo test -p sift` (expected: PASS)
**Commit**: `fix(scan): sort receipt entries alphabetically before returning`
