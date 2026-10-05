import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { EXPECTED_PUBKEY } from "./check-release-config.mjs";
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

// ---- hardening cases ----

const b64 = (s) => Buffer.from(s).toString("base64");
const GOOD_KEY_LINE = "RWSTq9oXSNJZ1RZzO+L/WZGJbHoqYcNgeKJHAWqZ7kU+hy5/R3MycGvZ";
const setPubkey = (text) => editConf((c) => (c.plugins.updater.pubkey = b64(text)));

test("negative: createUpdaterArtifacts is the string \"true\"", () => {
  const r = run(editConf((c) => (c.bundle.createUpdaterArtifacts = "true")));
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL bundle\.createUpdaterArtifacts/);
});

test("a version line in a table before [package] is not the package version", () => {
  const r = run(
    edit("src-tauri/Cargo.toml", (t) =>
      `[dependencies.x]\nversion = "9.9.9"\n\n${t}`,
    ),
  );
  assert.equal(r.status, 0, r.stdout + r.stderr);
});

test("negative: duplicate sift entries in Cargo.lock", () => {
  const r = run(
    edit("src-tauri/Cargo.lock", (t) =>
      `${t}\n[[package]]\nname = "sift"\nversion = "9.9.9"\n`,
    ),
  );
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL .*Cargo\.lock/);
});

test("negative: missing tauri.conf.json", () => {
  const r = run((dir) => rmSync(path.join(dir, "src-tauri/tauri.conf.json")));
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL .*tauri\.conf\.json/);
});

test("negative: package.json version has a v prefix", () => {
  const r = run(
    edit("package.json", (t) => t.replace(/"version": "[^"]*"/, '"version": "v0.1.4"')),
  );
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL package\.json version is semver/);
});

for (const bad of ["0.1.4-rc.1", "0.1.4+build.5"]) {
  test(`negative: ${bad} is not a plain MAJOR.MINOR.PATCH version`, () => {
    const r = run((dir) => {
      edit("package.json", (t) => t.replace(/"version": "[^"]*"/, `"version": "${bad}"`))(dir);
      edit("src-tauri/Cargo.toml", (t) => t.replace(/^version = "[^"]*"/m, `version = "${bad}"`))(dir);
      edit("src-tauri/Cargo.lock", (t) =>
        t.replace(/(name = "sift"\nversion = )"[^"]*"/, `$1"${bad}"`),
      )(dir);
    });
    assert.equal(r.status, 1);
    assert.match(r.stdout, /FAIL package\.json version is semver/);
  });
}

test("negative: valid base64 that is not a minisign key", () => {
  const r = run(setPubkey("this is just some random text\nnot a key\n"));
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL plugins\.updater\.pubkey is a minisign/);
});

test("negative: minisign key whose second line does not start with RW", () => {
  const r = run(
    setPubkey("untrusted comment: minisign public key: D559D24817DAAB93\nXXSTq9oX\n"),
  );
  assert.equal(r.status, 1);
});

test("negative: a different minisign key id", () => {
  const r = run(
    setPubkey(`untrusted comment: minisign public key: 0123456789ABCDEF\n${GOOD_KEY_LINE}\n`),
  );
  assert.equal(r.status, 1);
  // Same key bytes under another comment: the pin catches it, the bytes still
  // carry the production id.
  assert.match(r.stdout, /FAIL plugins\.updater\.pubkey equals the pinned production key/);
});

// A minisign public key file for arbitrary key material: "Ed" + id + 32 bytes.
const keyLine = (idHexBigEndian, keyBytes) =>
  Buffer.concat([
    Buffer.from("Ed"),
    Buffer.from(idHexBigEndian, "hex").reverse(),
    keyBytes,
  ]).toString("base64");
const PROD_ID = "D559D24817DAAB93";
const PROD_COMMENT = `untrusted comment: minisign public key: ${PROD_ID}`;

// Verifier's spoof: a fresh key wrapped under the production comment line.
test("negative: spoof, a fresh key under the production comment", () => {
  const r = run(
    setPubkey(`${PROD_COMMENT}\n${keyLine("0123456789ABCDEF", randomBytes(32))}\n`),
  );
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL plugins\.updater\.pubkey equals the pinned production key/);
  assert.match(r.stdout, /FAIL plugins\.updater\.pubkey key id \(from the key bytes\)/);
});

test("negative: spoof, different key material that reuses the production key id", () => {
  const r = run(setPubkey(`${PROD_COMMENT}\n${keyLine(PROD_ID, randomBytes(32))}\n`));
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL plugins\.updater\.pubkey equals the pinned production key/);
});

test("negative: key material of the wrong length", () => {
  const r = run(
    setPubkey(`${PROD_COMMENT}\n${Buffer.concat([Buffer.from("Ed"), randomBytes(20)]).toString("base64")}\n`),
  );
  assert.equal(r.status, 1);
});

test("the pinned constants describe the key in tauri.conf.json", () => {
  const conf = JSON.parse(readFileSync(path.join(repo, "src-tauri/tauri.conf.json"), "utf8"));
  assert.equal(conf.plugins.updater.pubkey, EXPECTED_PUBKEY);
  const r = run();
  assert.match(r.stdout, /ok   pinned EXPECTED_PUBKEY embeds key id D559D24817DAAB93/);
});

test("negative: https endpoint outside this repository's releases", () => {
  const r = run(
    editConf(
      (c) => (c.plugins.updater.endpoints = ["https://example.com/latest.json"]),
    ),
  );
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL plugins\.updater\.endpoints/);
});

test("--root followed by a flag is a usage error on stderr, not a path", () => {
  const r = spawnSync(process.execPath, [checker, "--root", "--print-version"], {
    encoding: "utf8",
  });
  assert.equal(r.status, 2);
  assert.equal(r.stdout, "");
  assert.match(r.stderr, /usage/);
});

test("negative: pubkey comment line is not the minisign public key header", () => {
  const r = run(
    setPubkey(`untrusted comment: signature from tauri secret key: D559D24817DAAB93\n${GOOD_KEY_LINE}\n`),
  );
  assert.equal(r.status, 1);
  assert.match(r.stdout, /FAIL plugins\.updater\.pubkey is a minisign/);
});
