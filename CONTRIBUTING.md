# Contributing

## Commit convention

Every commit subject has the form `type(scope): description`. The rules live in
`.cz.toml` and are enforced by [commitizen](https://commitizen-tools.github.io/commitizen/)
in a local `commit-msg` hook and, later, in CI.

### Types

| Type | Bump | Use for |
|:-----|:-----|:--------|
| `feat` | MINOR | a new user-visible capability |
| `fix` | PATCH | behaviour that was wrong |
| `perf` | PATCH | speed or memory improvements, with a measured before/after |
| `refactor` | none | restructuring without changing observable behaviour |
| `test` | none | tests only |
| `docs` | none | README, CONTRIBUTING, code comments |
| `build` | none | manifests, lockfiles, bundler and toolchain config |
| `ci` | none | GitHub Actions workflows |
| `chore` | none | housekeeping that fits nowhere else |
| `rdm` | none | files under `__roadmap__/` |
| `report` | none | files under `__reports__/` |
| `release` | none | written only by `cz bump`; never author one by hand |

A breaking change bumps MAJOR (MINOR while the version is below 1.0.0). Mark it
with a `!` after the scope (`feat(updater)!: ...`), a `BREAKING CHANGE:` footer,
or both; either one is enough, and it works on any type, not only `feat`. The
"Bump" column shows the bump without it. Known limitation: `fix(x)!:` is listed
under "Fixed" in the changelog, not as breaking.

### Scope

The scope is mandatory and kebab-case: lowercase letters, digits and hyphens,
starting with a letter and not ending in a hyphen. Name the area touched, not
the file. Scopes already in use: `frontend`, `backend`, `release`, `updater`,
`config`, `toast`, `ui`, `settings`, `fields`, `tabs`, `deps`, `ci`, `worktree`.
A roadmap leaf's own name is the scope for its commits
(`rdm(update-flow-hardening): ...`).

The scopes `worktree`, `roadmap` and `commit-convention` never reach a user:
they neither bump the version nor appear in the changelog, even on a `feat` or
`fix`.

### Subject

- Imperative mood, starts with a lowercase letter, digit or backtick.
- No trailing period.
- At most 100 characters for the whole first line.
- No leading space.

### Body

Separate it from the subject with a blank line. Add one when the reason is not
obvious. It explains WHY the change was made; the diff already shows what
changed. Wrap it at about 72 columns.

No line of a body may start like a typed subject (`feat(scope): ...`, optionally
indented). commitizen scans every line of a message for bumps, so such a line
would bump the version no matter what the subject says. `BREAKING CHANGE:` and
trailers such as `Co-Authored-By:` are fine. This applies to every commit.

### Merge commits

`Merge ...`, `Revert ...`, `fixup! ...`, `squash! ...` and `amend! ...` subjects
are exempt from the subject check, and the hook does not lint merge bodies, so
write the body as WHY prose and never start a line with `type(scope):`. The same
bump scan reads it. GitHub's merge button puts the pull request title in the
merge body, so the PR title's type classifies the merge: CI lints PR titles for
that reason.

### Install the hook

Once per clone:

```sh
bash scripts/install-hooks.sh
```

It writes a `commit-msg` stub into the shared git hooks directory, and the
stub runs the `.githooks/commit-msg` of the checkout you commit in. One install
covers every worktree. The hook needs [uv](https://docs.astral.sh/uv/) and
refuses the commit without it, so a missing tool is never a silent pass.

To check the validator itself, run `bash scripts/probe_cz_check.sh`.

## Gates

CI (`.github/workflows/ci.yml`, workflow "CI") runs four jobs on every pull
request and every push to `main`. Run the same commands locally before opening a
PR:

| Job id | Check name | Local command |
|:-------|:-----------|:--------------|
| `commits` | Commit messages | `uvx --from commitizen==4.19.1 cz check --rev-range <remote>/main..HEAD`, then `bash scripts/probe_cz_check.sh` (CI also lints the PR title) |
| `config` | Release config | `node scripts/check-release-config.mjs` and `SIFT_REQUIRE_SIGNING_TOOLS=1 node --test scripts/*.test.mjs` (needs `minisign` and `pnpm install`) |
| `frontend` | Frontend | `pnpm check && pnpm build && node --test src/lib/stores/*.test.ts` |
| `rust` | Rust | `pnpm build && (cd src-tauri && cargo test --lib)` |

`<remote>` is your remote's name. This checkout calls it `LittleCoinCoin`;
CI calls it `origin`. `git config branch.main.remote` prints yours.

Two end-to-end suites cover the update UX. CI does not run them; run them
before a release, or when you touch `src/lib/stores/updater*`:

- `pnpm e2e:ui` runs 13 browser scenarios against the real store and toast
  component with headless Chrome (about 20 s). To watch one, start the
  `sift-e2e-ui` preview and open `/e2e.html?scenario=<name>`.
- `pnpm e2e:native` builds an old and a new signed app with a throwaway key,
  updates one into the other through a local mock server, and checks the
  result on disk (a few minutes). It never touches `/Applications`, your real
  keychain items or the production signing key.

Never add a dependency without saying so in the PR: every roadmap leaf gates
on `pnpm-lock.yaml` staying unchanged.

## Releasing

Releases ship to every installed copy: Sift checks
`releases/latest/download/latest.json` about 4 s after launch and from
**Sift → Check for Updates…**. That is why a release is cut through a PR and
verified before it goes public.

1. From an up-to-date `main` with a clean tree, run `scripts/release-pr.sh`.
   It branches `release/next`, runs `cz bump --changelog`, deletes the local
   tag cz creates (CI is the only tag author), pushes the branch and opens a
   PR titled `release(sift): v<X.Y.Z>`. When nothing is releasable (cz exit 3
   or 21) it says so and exits 0.
2. Review the new `CHANGELOG.md` section in the PR. You may edit it before
   merging. Never edit a released section.
3. Merge with a **merge commit**. Squash and rebase merges are disabled.
   The merge is the release decision.
4. `.github/workflows/release.yml` then runs on `main`:
   - `decide`: releases only if neither a tag `v<X.Y.Z>` nor a published release
     exists. Every other push to `main` is a no-op.
   - `create-release`: a draft at the merge commit, with the CHANGELOG section
     and an install note.
   - `build`: Apple Silicon and Intel bundles, signed with the updater key from
     repository secrets.
   - `assemble-verify`: checks both signatures with minisign against the
     production public key pinned in `scripts/check-release-config.mjs`, runs
     `lipo` on the executable and the bundled pdfium, checks the bundle version,
     then writes and uploads `latest.json`.
   - `publish`: re-checks the verified bytes, publishes, and smoke-tests the
     public `latest.json` anonymously. If the smoke test fails, the release is
     yanked back to draft.

Pull requests that touch the release machinery rehearse this whole pipeline
under a `dryrun-<run id>` draft, which is deleted afterwards and never served.
A `workflow_dispatch` run defaults to the same dry run.

### Yanking a release

`gh release edit v<X.Y.Z> --draft=true` makes `latest` fall back to the
previous release. Copies that already installed it stay put, because the
updater never downgrades. The tag that publishing created remains, so
`decide` will not release that version again: ship the fix as the next patch
through a new release PR. Only if nothing was ever served may you delete the
remote tag and re-run the workflow with `dry_run=false`.

### Known limitations

- If another merge lands on `main` while a release run is still queued, the
  release builds from the later commit.
- Same-repository PRs that touch the release machinery run with the signing
  key, by design, so rehearsals are real. A GitHub Environment with required
  reviewers would add a gate.
- A runner killed mid-rehearsal can leave a `dryrun-*` draft behind. Delete it
  by hand; drafts are never served.
- The updater key is pinned by value. Rotating it means changing
  `EXPECTED_PUBKEY` in `scripts/check-release-config.mjs` on purpose, and
  installed copies only trust the old key.
