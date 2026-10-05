#!/usr/bin/env node
// Assembles and verifies the updater manifest `latest.json` (Node >= 22, no
// dependencies; needs the `minisign` binary on PATH).
//
// Usage:
//   node scripts/build-latest-json.mjs --assets <dir> --version <X.Y.Z> \
//     --tag v<X.Y.Z> --repo <owner/repo> --conf <tauri.conf.json> \
//     --out <file> [--notes-file <file>]
//
// <dir> holds the signed updater assets of one release, exactly as tauri-action
// uploads them: `Sift_<V>_aarch64.app.tar.gz` and `Sift_<V>_x64.app.tar.gz`,
// each next to its `.sig`. The script:
//   1. decodes `plugins.updater.pubkey` (base64 of a minisign .pub file);
//   2. verifies every tarball against its `.sig` (base64 of a minisign
//      signature file) with that key, failing on the first mismatch;
//   3. maps the tarballs to `darwin-aarch64` and `darwin-x86_64`, failing when
//      one is missing, duplicated or unrecognised;
//   4. writes the manifest, only after every check passed.
//
// The signature proves the bytes come from the holder of the updater key. It
// does not prove which CPU architecture a tarball holds: the platform key is
// derived from the file name, so the release workflow also runs `lipo` on the
// extracted apps.
//
// Exit codes: 0 manifest written, 1 verification or assembly failed (nothing
// written), 2 bad usage.

import { spawnSync } from "node:child_process";
import {
  mkdtempSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const BASE64 = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/;
const REPO = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/;

// tauri-action names the updater tarballs `<productName>_<version>_<arch>.app.tar.gz`
// with arch `aarch64` for Apple Silicon and `x64` for Intel (measured from
// tauri-action v1.0.0 src/utils.ts and src/build.ts). Any other arch token
// (universal, x86_64, ...) is rejected rather than guessed.
const TARBALL = /^(.+)_(\d+\.\d+\.\d+)_(aarch64|x64)\.app\.tar\.gz$/;
const PLATFORM_OF_ARCH = { aarch64: "darwin-aarch64", x64: "darwin-x86_64" };
const PLATFORM_KEYS = Object.values(PLATFORM_OF_ARCH);

/**
 * Decodes the base64 wrapper that tauri puts around a minisign file and checks
 * the layout, returning the minisign file text (newline-terminated).
 *   kind "pub": 2 lines, `untrusted comment: ...` then key material "RW..."
 *   kind "sig": 4 lines, `untrusted comment: ...`, signature, `trusted
 *               comment: ...`, global signature
 * Trailing whitespace on the base64 text is ignored; anything else that is not
 * strict base64 throws.
 */
export function decodeMinisign(b64, kind) {
  if (kind !== "pub" && kind !== "sig") {
    throw new Error(`decodeMinisign: unknown kind ${JSON.stringify(kind)}`);
  }
  const text = typeof b64 === "string" ? b64.trim() : "";
  if (text.length === 0 || !BASE64.test(text)) {
    throw new Error(`${kind} is not non-empty base64`);
  }
  const decoded = Buffer.from(text, "base64").toString("utf8");
  const lines = decoded.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  if (!(lines[0] ?? "").startsWith("untrusted comment:")) {
    throw new Error(`${kind} decodes to a file with no "untrusted comment:" first line`);
  }
  if (kind === "pub") {
    if (lines.length !== 2 || !lines[1].startsWith("RW")) {
      throw new Error('pub does not decode to a 2-line minisign public key ("RW..." on line 2)');
    }
  } else if (lines.length !== 4 || !lines[2].startsWith("trusted comment:")) {
    throw new Error("sig does not decode to a 4-line minisign signature with a trusted comment");
  }
  return `${lines.join("\n")}\n`;
}

/**
 * Verifies `tarball` against the base64 `.sig` text with the minisign public
 * key text `pubText`. Throws on any mismatch, including a missing minisign
 * binary (fail closed). `tmp` is a scratch directory the caller owns.
 */
export function verifyAsset({ tarball, sigB64, pubText, tmp }) {
  const sigText = decodeMinisign(sigB64, "sig");
  const pubFile = path.join(tmp, "updater.pub");
  const sigFile = path.join(tmp, "asset.minisig");
  writeFileSync(pubFile, pubText);
  writeFileSync(sigFile, sigText);
  const r = spawnSync("minisign", ["-V", "-m", tarball, "-p", pubFile, "-x", sigFile], {
    encoding: "utf8",
  });
  if (r.error) {
    throw new Error(`cannot run minisign (is it installed?): ${r.error.message}`);
  }
  if (r.status !== 0) {
    const why = `${r.stderr ?? ""}${r.stdout ?? ""}`.trim();
    throw new Error(
      `${path.basename(tarball)}: signature does not verify against plugins.updater.pubkey` +
        (why ? ` (${why})` : ""),
    );
  }
}

/**
 * Maps a list of file names to { "darwin-aarch64": {tarball, sig}, "darwin-x86_64": ... }.
 * Throws when a tarball does not look like tauri-action's output for `version`,
 * when two tarballs claim one platform, when a platform is missing, or when a
 * tarball and its `.sig` do not pair up one to one.
 */
export function mapPlatforms(names, version) {
  const tarballs = names.filter((n) => n.endsWith(".app.tar.gz"));
  const sigs = names.filter((n) => n.endsWith(".app.tar.gz.sig"));
  const mapped = {};
  for (const name of tarballs) {
    const m = TARBALL.exec(name);
    if (!m) throw new Error(`unrecognised updater asset name: ${name}`);
    if (m[2] !== version) {
      throw new Error(`${name} is version ${m[2]}, expected ${version}`);
    }
    const key = PLATFORM_OF_ARCH[m[3]];
    if (mapped[key]) {
      throw new Error(`ambiguous ${key}: both ${mapped[key].tarball} and ${name}`);
    }
    if (!sigs.includes(`${name}.sig`)) {
      throw new Error(`${name} has no ${name}.sig`);
    }
    mapped[key] = { tarball: name, sig: `${name}.sig` };
  }
  for (const s of sigs) {
    if (!tarballs.includes(s.slice(0, -".sig".length))) {
      throw new Error(`${s} has no matching tarball`);
    }
  }
  const missing = PLATFORM_KEYS.filter((k) => !mapped[k]);
  if (missing.length > 0) {
    throw new Error(`missing updater asset for: ${missing.join(", ")}`);
  }
  return mapped;
}

const USAGE =
  "usage: build-latest-json.mjs --assets <dir> --version <X.Y.Z> --tag v<X.Y.Z> " +
  "--repo <owner/repo> --conf <tauri.conf.json> --out <file> [--notes-file <file>]";
const OPTIONS = ["assets", "version", "tag", "repo", "conf", "out", "notes-file"];
const REQUIRED = OPTIONS.filter((o) => o !== "notes-file");

function parseArgs(argv) {
  const opts = {};
  for (let i = 0; i < argv.length; i++) {
    const name = argv[i].startsWith("--") ? argv[i].slice(2) : "";
    if (!OPTIONS.includes(name) || i + 1 >= argv.length || argv[i + 1].startsWith("--")) {
      throw new Error(`unknown or incomplete argument: ${argv[i]}`);
    }
    if (name in opts) throw new Error(`duplicate argument: ${argv[i]}`);
    opts[name] = argv[++i];
  }
  for (const r of REQUIRED) {
    if (!(r in opts)) throw new Error(`missing --${r}`);
  }
  return opts;
}

export function main(argv = process.argv.slice(2)) {
  let opts;
  try {
    opts = parseArgs(argv);
  } catch (e) {
    console.error(`${e.message}\n${USAGE}`);
    return 2;
  }
  const { assets, version, tag, repo, conf, out } = opts;
  if (!SEMVER.test(version)) {
    console.error(`--version must be MAJOR.MINOR.PATCH, got ${JSON.stringify(version)}\n${USAGE}`);
    return 2;
  }
  if (tag !== `v${version}`) {
    // A rehearsal tag in the manifest would point installed apps at a
    // release that is deleted right after the run.
    console.error(`--tag must be v${version}, got ${JSON.stringify(tag)}`);
    return 2;
  }
  if (!REPO.test(repo)) {
    console.error(`--repo must look like owner/repo, got ${JSON.stringify(repo)}`);
    return 2;
  }

  let tmp;
  try {
    const pubText = decodeMinisign(
      JSON.parse(readFileSync(conf, "utf8")).plugins?.updater?.pubkey,
      "pub",
    );
    const mapped = mapPlatforms(readdirSync(assets), version);
    tmp = mkdtempSync(path.join(tmpdir(), "latest-json-"));
    const platforms = {};
    for (const key of PLATFORM_KEYS) {
      const { tarball, sig } = mapped[key];
      const sigB64 = readFileSync(path.join(assets, sig), "utf8");
      verifyAsset({ tarball: path.join(assets, tarball), sigB64, pubText, tmp });
      platforms[key] = {
        // Verbatim, minus a stray trailing newline the updater's base64
        // decoder would reject; tauri's own .sig files have none.
        signature: sigB64.trim(),
        url: `https://github.com/${repo}/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(tarball)}`,
      };
      console.error(`ok   ${key}: ${tarball} verifies`);
    }
    const notes = opts["notes-file"] ? readFileSync(opts["notes-file"], "utf8").trim() : "";
    const manifest = { version, notes, pub_date: new Date().toISOString(), platforms };
    writeFileSync(out, `${JSON.stringify(manifest, null, 2)}\n`);
    console.error(`wrote ${out}`);
    return 0;
  } catch (e) {
    console.error(`FAIL ${e.message}`);
    return 1;
  } finally {
    if (tmp) rmSync(tmp, { recursive: true, force: true });
  }
}

// Compare real paths: argv[1] may reach this file through a symlink (e.g.
// macOS /var -> /private/var), which would otherwise skip main() and exit 0.
function isEntryPoint() {
  if (!process.argv[1]) return false;
  try {
    return (
      realpathSync(process.argv[1]) ===
      realpathSync(fileURLToPath(import.meta.url))
    );
  } catch {
    return false;
  }
}

if (isEntryPoint()) {
  process.exitCode = main();
}
