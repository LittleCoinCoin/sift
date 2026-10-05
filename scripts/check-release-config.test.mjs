import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const checker = path.join(repo, "scripts", "check-release-config.mjs");
const FILES = [
  "package.json",
  "src-tauri/Cargo.toml",
  "src-tauri/Cargo.lock",
  "src-tauri/tauri.conf.json",
];

// Copies the real release files into a temp root, applies `mutate(dir)`, runs
// the checker against it and cleans up.
function run(mutate, args = []) {
  const dir = mkdtempSync(path.join(os.tmpdir(), "release-config-"));
  try {
    for (const f of FILES) {
      mkdirSync(path.dirname(path.join(dir, f)), { recursive: true });
      cpSync(path.join(repo, f), path.join(dir, f));
    }
    mutate?.(dir);
    const r = spawnSync(process.execPath, [checker, "--root", dir, ...args], {
      encoding: "utf8",
    });
    return { status: r.status, stdout: r.stdout, stderr: r.stderr };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const edit = (rel, fn) => (dir) => {
  const p = path.join(dir, rel);
  const before = readFileSync(p, "utf8");
  const after = fn(before);
  assert.notEqual(after, before, `mutation of ${rel} changed nothing`);
  writeFileSync(p, after);
};
const editConf = (fn) =>
  edit("src-tauri/tauri.conf.json", (text) => {
    const conf = JSON.parse(text);
    fn(conf);
    return JSON.stringify(conf, null, 2);
  });

test("positive: the real repo files pass", () => {
  const r = run();
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.doesNotMatch(r.stdout, /FAIL/);
});

test("--print-version prints only the version on success", () => {
  const pkg = JSON.parse(readFileSync(path.join(repo, "package.json"), "utf8"));
  const r = run(undefined, ["--print-version"]);
  assert.equal(r.status, 0);
  assert.equal(r.stdout, `${pkg.version}\n`);
});

test("--print-version prints nothing on stdout when a check fails", () => {
  const r = run(
    editConf((c) => (c.bundle.createUpdaterArtifacts = false)),
    ["--print-version"],
  );
  assert.equal(r.status, 1);
  assert.equal(r.stdout, "");
  assert.match(r.stderr, /createUpdaterArtifacts/);
});

test("unknown flag exits 2", () => {
  assert.equal(run(undefined, ["--bogus"]).status, 2);
});

test("negative: Cargo.toml version drift", () => {
  const r = run(
    edit("src-tauri/Cargo.toml", (t) =>
      t.replace(/^version = "[^"]*"/m, 'version = "9.9.9"'),
    ),
  );
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL .*Cargo\.toml/);
});

test("negative: Cargo.lock sift entry drift", () => {
  const r = run(
    edit("src-tauri/Cargo.lock", (t) =>
      t.replace(/(name = "sift"\nversion = )"[^"]*"/, '$1"9.9.9"'),
    ),
  );
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL .*Cargo\.lock/);
});

test("negative: package.json version moves, manifests do not", () => {
  const r = run(
    edit("package.json", (t) => t.replace(/"version": "[^"]*"/, '"version": "9.9.9"')),
  );
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL .*Cargo\.toml/);
  assert.match(r.stdout, /FAIL .*Cargo\.lock/);
});

test("negative: package.json version is not semver", () => {
  const r = run(
    edit("package.json", (t) => t.replace(/"version": "[^"]*"/, '"version": "1.2"')),
  );
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL package\.json version is semver/);
});

test("negative: createUpdaterArtifacts false", () => {
  const r = run(editConf((c) => (c.bundle.createUpdaterArtifacts = false)));
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL bundle\.createUpdaterArtifacts/);
});

test("negative: createUpdaterArtifacts missing", () => {
  const r = run(editConf((c) => delete c.bundle.createUpdaterArtifacts));
  assert.equal(r.status, 1);
});

test("negative: empty pubkey", () => {
  const r = run(editConf((c) => (c.plugins.updater.pubkey = "")));
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL plugins\.updater\.pubkey/);
});

test("negative: pubkey that is not base64", () => {
  const r = run(editConf((c) => (c.plugins.updater.pubkey = "not base64!")));
  assert.equal(r.status, 1);
});

test("negative: http:// endpoint", () => {
  const r = run(
    editConf((c) =>
      c.plugins.updater.endpoints.push("http://example.com/latest.json"),
    ),
  );
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL plugins\.updater\.endpoints/);
});

test("negative: no endpoints", () => {
  const r = run(editConf((c) => (c.plugins.updater.endpoints = [])));
  assert.equal(r.status, 1);
});

test("negative: literal tauri.conf.json version", () => {
  const r = run(editConf((c) => (c.version = "0.1.4")));
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL tauri\.conf\.json version/);
});

test("negative: missing Cargo.lock is a failure, not a crash", () => {
  const r = run((dir) => rmSync(path.join(dir, "src-tauri/Cargo.lock")));
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL .*Cargo\.lock/);
});

// Regression: when argv[1] reached the script through a symlink the entry
// guard was false, main() never ran, and the checker exited 0 doing nothing.
test("runs through a symlinked scripts directory", () => {
  const dir = mkdtempSync(path.join(os.tmpdir(), "release-config-link-"));
  try {
    const link = path.join(dir, "scripts-link");
    symlinkSync(path.join(repo, "scripts"), link);
    const linked = path.join(link, "check-release-config.mjs");
    const run = (...args) =>
      spawnSync(process.execPath, [linked, "--root", repo, ...args], {
        encoding: "utf8",
      });
    const pkg = JSON.parse(readFileSync(path.join(repo, "package.json"), "utf8"));
    const ok = run("--print-version");
    assert.equal(ok.status, 0);
    assert.equal(ok.stdout, `${pkg.version}\n`);
    assert.equal(run("--bogus").status, 2);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
