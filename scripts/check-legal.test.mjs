import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { checkLegal } from './check-legal.mjs';
const fixture = JSON.parse(readFileSync(new URL('../tests/fixtures/legal-readiness.json', import.meta.url)));
function merge(base, patch) {
  if (base && patch && typeof base === 'object' && typeof patch === 'object') {
    for (const [key, value] of Object.entries(patch)) base[key] = merge(base[key], value);
    return base;
  }
  return structuredClone(patch);
}
for (const item of fixture.cases) {
  test(item.name, () => {
    const manifest = merge(structuredClone(fixture.manifest), item.patch);
    assert.equal(checkLegal(manifest, item.terms ?? 'Final test terms', item.privacy ?? 'Final test privacy').ready, item.ready);
  });
}
