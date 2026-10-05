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
or both; either one is enough.

### Scope

The scope is mandatory and kebab-case: lowercase letters, digits and hyphens,
starting with a letter. Name the area touched, not the file. Scopes already in
use: `frontend`, `backend`, `release`, `updater`, `config`, `toast`, `ui`,
`settings`, `fields`, `tabs`, `deps`, `ci`, `worktree`. A roadmap leaf's own name
is the scope for its commits (`rdm(update-flow-hardening): ...`).

The scopes `worktree`, `roadmap` and `commit-convention` never reach a user, so
they are kept out of the changelog even on a `feat` or `fix`.

### Subject

- Imperative mood, starts with a lowercase letter, digit or backtick.
- No trailing period.
- At most 100 characters for the whole first line.

### Body

Add a body when the reason is not obvious. It explains WHY the change was made;
the diff already shows what changed. Wrap it at about 72 columns.

### Merge commits

`Merge ...` subjects written by git, GitHub and `Revert ...`, `fixup!` and
`squash!` are exempt from the subject check. Write the body of a merge as WHY
prose and never start a line with `type(scope):`. commitizen would read such a
line as a change to bump or list in the changelog.

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
