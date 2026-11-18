import test from 'node:test';
import assert from 'node:assert/strict';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { checkBundle, importSource, peImports, pickBackendBuild } from './check-windows-bundle.mjs';

const systemDir = path.join(process.env.SystemRoot ?? 'C:\\Windows', 'System32');
// A Microsoft-signed image that imports the VC runtime: present wherever the redistributable is.
const sample = path.join(systemDir, 'msvcp140.dll');
const onWindows = { skip: process.platform !== 'win32' || !existsSync(sample) ? 'needs Windows with msvcp140.dll' : false };

function scratch(t) {
  const dir = mkdtempSync(path.join(tmpdir(), 'lectus-bundle-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}
function put(file, bytes) {
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, bytes);
}

test('peImports reads the import table of a real PE image', onWindows, () => {
  const names = peImports(readFileSync(sample)).map(n => n.toLowerCase());
  assert.ok(names.includes('vcruntime140.dll'), names.join(', '));
  assert.ok(names.includes('kernel32.dll') || names.some(n => n.startsWith('api-ms-')), names.join(', '));
});

test('peImports rejects files that are not PE images', () => {
  assert.throws(() => peImports(Buffer.from('not a dll at all, just some text padding it out')), /MZ/);
});

test('importSource never counts a redistributable runtime as inbox', onWindows, () => {
  assert.equal(importSource('MSVCP140.dll', new Set(), systemDir), null);
  assert.equal(importSource('VCOMP140.DLL', new Set(), systemDir), null);
  assert.equal(importSource('vcruntime140_1.dll', new Set(), systemDir), null);
  assert.equal(importSource('MSVCP140.dll', new Set(['msvcp140.dll']), systemDir), 'shipped');
  assert.equal(importSource('KERNEL32.dll', new Set(), systemDir), 'inbox');
  assert.equal(importSource('api-ms-win-crt-runtime-l1-1-0.dll', new Set(), systemDir), 'api-set');
  assert.equal(importSource('vulkan-1.dll', new Set(), systemDir), 'driver');
  assert.equal(importSource('lectus-made-up.dll', new Set(), systemDir), null);
});

test('checkBundle fails when the exe imports a runtime the bundle does not ship', onWindows, t => {
  const dir = scratch(t);
  put(path.join(dir, 'target/release/chirp.exe'), readFileSync(sample));
  const bare = checkBundle({ tauriDir: dir, resources: {}, systemDir });
  assert.equal(bare.ok, false);
  assert.ok(bare.issues.some(i => /chirp\.exe imports vcruntime140\.dll/i.test(i)), bare.issues.join('\n'));

  copyFileSync(path.join(systemDir, 'vcruntime140.dll'), path.join(dir, 'target/release/vcruntime140.dll'));
  const shipped = checkBundle({ tauriDir: dir, resources: { 'target/release/vcruntime140.dll': 'vcruntime140.dll' }, systemDir });
  assert.ok(!shipped.issues.some(i => /chirp\.exe imports vcruntime140\.dll/i.test(i)), shipped.issues.join('\n'));
});

test('a resource that matches nothing is an issue', onWindows, t => {
  const dir = scratch(t);
  put(path.join(dir, 'target/release/chirp.exe'), readFileSync(sample));
  const result = checkBundle({ tauriDir: dir, resources: { 'target/release/missing.dll': 'missing.dll' }, systemDir });
  assert.ok(result.issues.includes('resource target/release/missing.dll matches no file'));
});

test('pickBackendBuild takes the build whose ggml-base.dll ships at the root', t => {
  const dir = scratch(t);
  const build = name => path.join(dir, 'target/release/build', name, 'out');
  put(path.join(dir, 'target/release/ggml-base.dll'), 'base-A');
  put(path.join(build('llama-cpp-sys-2-a'), 'bin/ggml-base.dll'), 'base-A');
  put(path.join(build('llama-cpp-sys-2-a'), 'backends/ggml-vulkan.dll'), 'vk-A');
  put(path.join(build('llama-cpp-sys-2-b'), 'bin/ggml-base.dll'), 'base-B');
  put(path.join(build('llama-cpp-sys-2-b'), 'backends/ggml-vulkan.dll'), 'vk-B');
  const files = [path.join(build('llama-cpp-sys-2-a'), 'backends/ggml-vulkan.dll'), path.join(build('llama-cpp-sys-2-b'), 'backends/ggml-vulkan.dll')];

  const issues = [];
  const warnings = [];
  assert.deepEqual(pickBackendBuild(dir, files, issues, warnings), [files[0]]);
  assert.deepEqual(issues, []);
  assert.equal(warnings.length, 1);

  // Root DLLs left over from an older llama build: nothing matches, so the bundle must stop.
  put(path.join(dir, 'target/release/ggml-base.dll'), 'base-old');
  const stale = [];
  assert.deepEqual(pickBackendBuild(dir, files, stale, []), []);
  assert.match(stale[0], /comes from none of the current llama-cpp-sys-2 builds/);
});
