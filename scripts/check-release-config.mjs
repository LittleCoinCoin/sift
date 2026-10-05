#!/usr/bin/env node
// Release-config consistency checker (Node >= 22, no dependencies).
//
// Usage:
//   node scripts/check-release-config.mjs [--root <dir>] [--print-version]
//
//   --root <dir>      repo root to inspect (default: the parent of scripts/)
//   --print-version   on success print only the package.json version
//                     (one line, no other output on stdout)
//
// Exit codes: 0 all checks pass, 1 at least one check failed, 2 bad usage.
// Human mode prints one line per check on stdout: "ok   <check>" or
// "FAIL <check>: <detail>". With --print-version, failures go to stderr.

import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SEMVER =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/;
const BASE64 = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/;

const defaultRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

function readText(root, rel) {
  return readFileSync(path.join(root, rel), "utf8");
}

// Returns the value of `version = "..."` inside the `[package]` table.
function cargoTomlVersion(text) {
  let inPackage = false;
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (line.startsWith("[")) {
      inPackage = line === "[package]";
      continue;
    }
    if (inPackage) {
      const m = /^version\s*=\s*"([^"]*)"\s*(?:#.*)?$/.exec(line);
      if (m) return m[1];
    }
  }
  throw new Error("no [package] version found");
}

// Returns the version of the single `name = "sift"` [[package]] entry.
function cargoLockSiftVersion(text) {
  const versions = [];
  for (const block of text.split(/^\[\[package\]\]\s*$/m).slice(1)) {
    const name = /^name\s*=\s*"([^"]*)"/m.exec(block);
    if (name && name[1] === "sift") {
      const v = /^version\s*=\s*"([^"]*)"/m.exec(block);
      versions.push(v ? v[1] : undefined);
    }
  }
  if (versions.length !== 1) {
    throw new Error(`expected exactly one sift entry, found ${versions.length}`);
  }
  if (versions[0] === undefined) throw new Error("sift entry has no version");
  return versions[0];
}

// Each source is read independently so one unreadable file does not hide the
// others. Every field is { value } or { error }.
export function readVersions(root = defaultRoot) {
  const attempt = (fn) => {
    try {
      return { value: fn() };
    } catch (e) {
      return { error: e.message };
    }
  };
  return {
    package: attempt(() => {
      const v = JSON.parse(readText(root, "package.json")).version;
      if (typeof v !== "string") throw new Error("version is not a string");
      return v;
    }),
    cargoToml: attempt(() =>
      cargoTomlVersion(readText(root, "src-tauri/Cargo.toml")),
    ),
    cargoLock: attempt(() =>
      cargoLockSiftVersion(readText(root, "src-tauri/Cargo.lock")),
    ),
  };
}

export function checkTauriConf(root = defaultRoot) {
  const results = [];
  const add = (name, ok, detail = "") => results.push({ name, ok, detail });
  let conf;
  try {
    conf = JSON.parse(readText(root, "src-tauri/tauri.conf.json"));
  } catch (e) {
    add("src-tauri/tauri.conf.json is readable JSON", false, e.message);
    return results;
  }
  add(
    'tauri.conf.json version is "../package.json"',
    conf.version === "../package.json",
    `got ${JSON.stringify(conf.version)}`,
  );
  const art = conf.bundle?.createUpdaterArtifacts;
  add(
    "bundle.createUpdaterArtifacts is true",
    art === true,
    `got ${JSON.stringify(art)}`,
  );
  const pubkey = conf.plugins?.updater?.pubkey;
  add(
    "plugins.updater.pubkey is non-empty base64",
    typeof pubkey === "string" && pubkey.length > 0 && BASE64.test(pubkey),
    `got ${JSON.stringify(pubkey)}`,
  );
  const endpoints = conf.plugins?.updater?.endpoints;
  const bad = Array.isArray(endpoints)
    ? endpoints.filter((e) => typeof e !== "string" || !e.startsWith("https://"))
    : null;
  add(
    "plugins.updater.endpoints are all https://",
    Array.isArray(endpoints) && endpoints.length > 0 && bad.length === 0,
    !Array.isArray(endpoints) || endpoints.length === 0
      ? "endpoints missing or empty"
      : `non-https: ${JSON.stringify(bad)}`,
  );
  return results;
}

function checkVersions(root) {
  const v = readVersions(root);
  const results = [];
  const add = (name, ok, detail = "") => results.push({ name, ok, detail });
  let version;
  if (v.package.error) {
    add("package.json version is semver", false, v.package.error);
  } else {
    version = v.package.value;
    add(
      "package.json version is semver",
      SEMVER.test(version),
      `got ${JSON.stringify(version)}`,
    );
  }
  for (const [key, label] of [
    ["cargoToml", "src-tauri/Cargo.toml [package] version"],
    ["cargoLock", 'src-tauri/Cargo.lock "sift" version'],
  ]) {
    if (v[key].error) {
      add(`${label} equals package.json`, false, v[key].error);
    } else {
      add(
        `${label} equals package.json`,
        v[key].value === version,
        `got ${JSON.stringify(v[key].value)}, package.json has ${JSON.stringify(version)}`,
      );
    }
  }
  return { results, version };
}

export function main(argv = process.argv.slice(2)) {
  let root = defaultRoot;
  let printVersion = false;
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--print-version") {
      printVersion = true;
    } else if (argv[i] === "--root" && i + 1 < argv.length) {
      root = path.resolve(argv[++i]);
    } else {
      console.error(
        `unknown or incomplete argument: ${argv[i]}\n` +
          "usage: check-release-config.mjs [--root <dir>] [--print-version]",
      );
      return 2;
    }
  }

  const { results: vr, version } = checkVersions(root);
  const results = [...vr, ...checkTauriConf(root)];
  const failed = results.filter((r) => !r.ok);

  if (printVersion && failed.length === 0) {
    console.log(version);
    return 0;
  }
  const out = printVersion ? console.error : console.log;
  for (const r of results) {
    out(r.ok ? `ok   ${r.name}` : `FAIL ${r.name}: ${r.detail}`);
  }
  return failed.length === 0 ? 0 : 1;
}

if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  process.exitCode = main();
}
