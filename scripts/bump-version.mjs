#!/usr/bin/env node
// Single-source version bump: rewrites package.json, src-tauri/Cargo.toml and
// src-tauri/tauri.conf.json so the release workflow's version guard passes.
// Usage: node scripts/bump-version.mjs 0.5.0
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error('Usage: node scripts/bump-version.mjs <semver>  (e.g. 0.5.0)');
  process.exit(1);
}

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const files = [
  { path: 'package.json', re: /("version":\s*")[^"]+(")/ },
  { path: 'src-tauri/tauri.conf.json', re: /("version":\s*")[^"]+(")/ },
  { path: 'src-tauri/Cargo.toml', re: /(^version\s*=\s*")[^"]+(")/m },
];

for (const { path, re } of files) {
  const full = resolve(root, path);
  const src = readFileSync(full, 'utf8');
  if (!re.test(src)) {
    console.error(`No version field found in ${path}`);
    process.exit(1);
  }
  writeFileSync(full, src.replace(re, `$1${version}$2`));
  console.log(`${path} -> ${version}`);
}
