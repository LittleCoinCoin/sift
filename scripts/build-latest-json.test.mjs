// Tests for build-latest-json.mjs. Fixtures are signed at test time with
// throwaway keys made by `pnpm tauri signer generate`, so the .sig and pubkey
// have exactly the shape of the production ones (base64 of a minisign file,
// prehashed signature). The production key is never used.
//
// Prerequisites: `minisign` on PATH and the repo's pnpm install. When either is
// missing the signing tests are skipped, unless SIFT_REQUIRE_SIGNING_TOOLS=1,
// which turns a missing tool into a failure (the release workflow sets it).

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { decodeMinisign, mapPlatforms, main } from "./build-latest-json.mjs";

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const script = path.join(repo, "scripts", "build-latest-json.mjs");
const realConf = path.join(repo, "src-tauri", "tauri.conf.json");
const V = "0.2.0";
const REPO = "LittleCoinCoin/sift";
const ARM = `Sift_${V}_aarch64.app.tar.gz`;
const X64 = `Sift_${V}_x64.app.tar.gz`;

const sh = (cmd, args, opts = {}) =>
  spawnSync(cmd, args, { cwd: repo, encoding: "utf8", ...opts });

const minisignOk = sh("minisign", ["-v"]).status === 0;
const tauriOk = sh("pnpm", ["tauri", "--version"]).status === 0;
const toolsOk = minisignOk && tauriOk;
if (!toolsOk && process.env.SIFT_REQUIRE_SIGNING_TOOLS === "1") {
  throw new Error(
    `signing tools missing (minisign: ${minisignOk}, pnpm tauri: ${tauriOk}) ` +
      "and SIFT_REQUIRE_SIGNING_TOOLS=1",
  );
}
const signing = {
  skip: toolsOk ? false : "minisign or `pnpm tauri` unavailable",
};

const work = mkdtempSync(path.join(os.tmpdir(), "latest-json-test-"));
process.on("exit", () => rmSync(work, { recursive: true, force: true }));

// A throwaway key: { priv, pub (base64, as in tauri.conf.json) }.
function makeKey(name) {
  const priv = path.join(work, name);
  const r = sh("pnpm", ["tauri", "signer", "generate", "--ci", "-p", "", "-w", priv]);
  assert.equal(r.status, 0, r.stderr);
  return { priv, pub: readFileSync(`${priv}.pub`, "utf8").trim() };
}

// Writes `<dir>/<name>` with `content` and signs it; returns the .sig path.
function signedAsset(dir, name, content, key) {
  const file = path.join(dir, name);
  writeFileSync(file, content);
  const r = sh("pnpm", ["tauri", "signer", "sign", "-f", key.priv, "-p", "", file]);
  assert.equal(r.status, 0, r.stderr);
  assert.ok(existsSync(`${file}.sig`), "signer wrote no .sig");
  return `${file}.sig`;
}

function confWith(pub) {
  const conf = JSON.parse(readFileSync(realConf, "utf8"));
  conf.plugins.updater.pubkey = pub;
  const p = path.join(work, `conf-${Math.random().toString(36).slice(2)}.json`);
  writeFileSync(p, JSON.stringify(conf));
  return p;
}

let counter = 0;
const keyA = toolsOk ? makeKey("a") : null;
const keyB = toolsOk ? makeKey("b") : null;

// Builds an assets dir with both platforms signed by `key` (default A).
function assetsDir(mutate, key = keyA) {
  const dir = path.join(work, `assets-${counter++}`);
  mkdirSync(dir);
  signedAsset(dir, ARM, "arm64 payload", key);
  signedAsset(dir, X64, "x86_64 payload", key);
  mutate?.(dir);
  return dir;
}

const pubkeyOf = (confPath) =>
  JSON.parse(readFileSync(confPath, "utf8")).plugins.updater.pubkey;

function argvFor(dir, conf, out, { extra = [], tag = `v${V}`, version = V } = {}) {
  return [
    "--assets", dir, "--version", version, "--tag", tag,
    "--repo", REPO, "--conf", conf, "--out", out, ...extra,
  ];
}

// In-process, with the pin set to `pin` (default: the conf's own key) so the
// throwaway-key fixtures can reach the signature checks. The command line has
// no such switch; `runCli` below uses the real pin.
function run(dir, { conf = confWith(keyA.pub), pin = pubkeyOf(conf), ...rest } = {}) {
  const out = path.join(work, `out-${counter++}.json`);
  const lines = [];
  const orig = console.error;
  console.error = (...a) => lines.push(a.join(" "));
  let status;
  try {
    status = main(argvFor(dir, conf, out, rest), { expectedPubkey: pin });
  } finally {
    console.error = orig;
  }
  return { status, stderr: lines.join("\n"), out, wrote: existsSync(out) };
}

// The real command line: the pin is the production key.
function runCli(dir, conf) {
  const out = path.join(work, `out-${counter++}.json`);
  const r = sh(process.execPath, [script, ...argvFor(dir, conf, out)]);
  return { status: r.status, stderr: r.stderr, out, wrote: existsSync(out) };
}

test("positive: both platforms are emitted with verbatim signatures and release URLs", signing, () => {
  const dir = assetsDir();
  const notes = path.join(work, "notes.md");
  writeFileSync(notes, "### Added\n\n- thing\n");
  const r = run(dir, { extra: ["--notes-file", notes] });
  assert.equal(r.status, 0, r.stderr);
  const m = JSON.parse(readFileSync(r.out, "utf8"));
  assert.deepEqual(Object.keys(m), ["version", "notes", "pub_date", "platforms"]);
  assert.equal(m.version, V);
  assert.equal(m.notes, "### Added\n\n- thing");
  assert.ok(!Number.isNaN(Date.parse(m.pub_date)));
  assert.deepEqual(Object.keys(m.platforms), ["darwin-aarch64", "darwin-x86_64"]);
  for (const [key, name] of [["darwin-aarch64", ARM], ["darwin-x86_64", X64]]) {
    assert.equal(m.platforms[key].signature, readFileSync(path.join(dir, `${name}.sig`), "utf8"));
    assert.equal(
      m.platforms[key].url,
      `https://github.com/${REPO}/releases/download/v${V}/${name}`,
    );
  }
  // The platform key follows the file name: the aarch64 entry carries the
  // signature of the aarch64 tarball, not the x64 one.
  assert.notEqual(m.platforms["darwin-aarch64"].signature, m.platforms["darwin-x86_64"].signature);
});

test("wrong key: a .sig made with another key exits 1 and writes nothing", signing, () => {
  const r = run(assetsDir(), { conf: confWith(keyB.pub) });
  assert.equal(r.status, 1);
  assert.equal(r.wrote, false);
  assert.match(r.stderr, /does not verify/);
});

test("the production pubkey rejects throwaway-signed assets", signing, () => {
  const r = run(assetsDir(), { conf: realConf });
  assert.equal(r.status, 1);
  assert.equal(r.wrote, false);
  assert.match(r.stderr, /does not verify/);
});

test("the command line pins the production key: a conf with any other key is refused", signing, () => {
  const r = runCli(assetsDir(), confWith(keyA.pub));
  assert.equal(r.status, 1);
  assert.equal(r.wrote, false);
  assert.match(r.stderr, /not the pinned production key/);
});

test("a spoofed conf (fresh key under the production comment) is refused before any signature check", signing, () => {
  const [comment] = Buffer.from(pubkeyOf(realConf), "base64").toString("utf8").split("\n");
  const spoof = Buffer.from(`${comment}\n${Buffer.from(keyA.pub, "base64").toString("utf8").split("\n")[1]}\n`).toString("base64");
  const r = runCli(assetsDir(), confWith(spoof));
  assert.equal(r.status, 1);
  assert.match(r.stderr, /not the pinned production key/);
});

test("one bad signature among two good ones fails the whole run", signing, () => {
  const dir = assetsDir((d) => signedAsset(d, X64, "x86_64 payload", keyB));
  const r = run(dir);
  assert.equal(r.status, 1);
  assert.equal(r.wrote, false);
  assert.match(r.stderr, new RegExp(X64.replaceAll(".", "\\.")));
});

test("a tampered tarball fails", signing, () => {
  const dir = assetsDir((d) => writeFileSync(path.join(d, ARM), "tampered"));
  const r = run(dir);
  assert.equal(r.status, 1);
  assert.equal(r.wrote, false);
});

test("swapped signatures fail: each tarball only verifies against its own .sig", signing, () => {
  const dir = assetsDir((d) => {
    copyFileSync(path.join(d, `${ARM}.sig`), path.join(d, "tmp.sig"));
    copyFileSync(path.join(d, `${X64}.sig`), path.join(d, `${ARM}.sig`));
    copyFileSync(path.join(d, "tmp.sig"), path.join(d, `${X64}.sig`));
    rmSync(path.join(d, "tmp.sig"));
  });
  const r = run(dir);
  assert.equal(r.status, 1);
  assert.equal(r.wrote, false);
});

test("missing platform: no x64 asset exits 1", signing, () => {
  const dir = assetsDir((d) => {
    rmSync(path.join(d, X64));
    rmSync(path.join(d, `${X64}.sig`));
  });
  const r = run(dir);
  assert.equal(r.status, 1);
  assert.equal(r.wrote, false);
  assert.match(r.stderr, /missing updater asset for: darwin-x86_64/);
});

test("missing platform: no aarch64 asset exits 1", signing, () => {
  const dir = assetsDir((d) => {
    rmSync(path.join(d, ARM));
    rmSync(path.join(d, `${ARM}.sig`));
  });
  const r = run(dir);
  assert.equal(r.status, 1);
  assert.match(r.stderr, /missing updater asset for: darwin-aarch64/);
});

test("a tarball without its .sig exits 1", signing, () => {
  const r = run(assetsDir((d) => rmSync(path.join(d, `${X64}.sig`))));
  assert.equal(r.status, 1);
  assert.match(r.stderr, /has no .*\.sig/);
});

test("an orphan .sig exits 1", signing, () => {
  const r = run(assetsDir((d) => copyFileSync(path.join(d, `${ARM}.sig`), path.join(d, `Sift_${V}_universal.app.tar.gz.sig`))));
  assert.equal(r.status, 1);
  assert.match(r.stderr, /no matching tarball/);
});

test("an extra tarball for an already mapped platform is ambiguous", signing, () => {
  const dir = assetsDir((d) => signedAsset(d, `Other_${V}_x64.app.tar.gz`, "dup", keyA));
  const r = run(dir);
  assert.equal(r.status, 1);
  assert.match(r.stderr, /ambiguous darwin-x86_64/);
});

test("an unrecognised arch token exits 1 instead of being guessed", signing, () => {
  const dir = assetsDir((d) => signedAsset(d, `Sift_${V}_universal.app.tar.gz`, "u", keyA));
  const r = run(dir);
  assert.equal(r.status, 1);
  assert.match(r.stderr, /unrecognised updater asset name/);
});

test("an asset of another version exits 1", signing, () => {
  const dir = assetsDir((d) => {
    rmSync(path.join(d, X64));
    rmSync(path.join(d, `${X64}.sig`));
    signedAsset(d, "Sift_0.1.4_x64.app.tar.gz", "old", keyA);
  });
  const r = run(dir);
  assert.equal(r.status, 1);
  assert.match(r.stderr, /expected 0\.2\.0/);
});

test("a malformed pubkey in the conf exits 1", signing, () => {
  const r = run(assetsDir(), { conf: confWith("not base64!") });
  assert.equal(r.status, 1);
  assert.equal(r.wrote, false);
});

test("a rehearsal tag is refused (exit 2) so it can never reach the manifest", signing, () => {
  const r = run(assetsDir(), { tag: "dryrun-123" });
  assert.equal(r.status, 2);
  assert.equal(r.wrote, false);
});

test("usage errors exit 2", () => {
  const exit = (args) => sh(process.execPath, [script, ...args]).status;
  assert.equal(exit([]), 2);
  assert.equal(exit(["--bogus", "x"]), 2);
  assert.equal(exit(["--assets"]), 2);
  const full = ["--assets", ".", "--tag", "v1.2.3", "--repo", REPO, "--conf", realConf, "--out", path.join(work, "x.json")];
  assert.equal(exit([...full, "--version", "1.2"]), 2);
  assert.equal(exit([...full, "--version", "1.2.3", "--repo", "nope"]), 2);
});

test("decodeMinisign accepts real tauri output and rejects look-alikes", signing, () => {
  const sigPath = signedAsset(work, `decode-${counter++}.tar.gz`, "x", keyA);
  const sigText = decodeMinisign(readFileSync(sigPath, "utf8"), "sig");
  assert.match(sigText, /^untrusted comment: signature from tauri secret key\nRU/);
  assert.match(decodeMinisign(keyA.pub, "pub"), /^untrusted comment: minisign public key: [0-9A-F]{16}\nRW/);
  // a signature is not a public key and vice versa
  assert.throws(() => decodeMinisign(keyA.pub, "sig"));
  assert.throws(() => decodeMinisign(readFileSync(sigPath, "utf8"), "pub"));
  assert.throws(() => decodeMinisign("", "pub"));
  assert.throws(() => decodeMinisign("***", "sig"));
  assert.throws(() => decodeMinisign(Buffer.from("hello\nworld\n").toString("base64"), "pub"));
});

test("mapPlatforms pairs tarballs with .sig files and fails closed", () => {
  const ok = [ARM, `${ARM}.sig`, X64, `${X64}.sig`, `Sift_${V}_aarch64.dmg`, "notes.txt"];
  assert.deepEqual(mapPlatforms(ok, V), {
    "darwin-aarch64": { tarball: ARM, sig: `${ARM}.sig` },
    "darwin-x86_64": { tarball: X64, sig: `${X64}.sig` },
  });
  assert.throws(() => mapPlatforms([], V), /missing updater asset/);
  assert.throws(() => mapPlatforms([ARM, `${ARM}.sig`], V), /darwin-x86_64/);
  assert.throws(() => mapPlatforms([...ok, `Sift_${V}_x86_64.app.tar.gz`], V), /unrecognised/);
});
