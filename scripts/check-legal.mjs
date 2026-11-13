/** Prevent distributing placeholder identity or draft agreements. Not a legal opinion. */
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
const root = fileURLToPath(new URL('..', import.meta.url));
export function validContact(value) {
  if (typeof value !== 'string' || /\s/u.test(value)) return false;
  const parts = value.split('@');
  if (parts.length !== 2 || !parts[0]) return false;
  const domain = parts[1];
  return domain.includes('.') && !domain.startsWith('.') && !domain.endsWith('.') && !domain.includes('..');
}
const present = value => typeof value === 'string' && value.trim().length > 0;

export function checkLegal(manifest, terms, privacy) {
  const issues = [];
  if (manifest?.publication_approved !== true) issues.push('Publication approval is not recorded.');
  if (manifest?.identity_disclosures_reviewed !== true) issues.push('Publisher disclosure requirements have not been resolved.');
  if (manifest?.third_party_notices_reviewed !== true) issues.push('Third-party and model-license obligations have not been reconciled for the shipped artifacts.');
  const publisher = manifest?.publisher;
  if (!['individual', 'company'].includes(publisher?.kind) || !present(publisher?.legal_name)) issues.push('Public legal publisher identity is unresolved.');
  if (!present(publisher?.country)) issues.push('Publisher country is missing.');
  for (const field of ['support_email', 'privacy_email']) {
    if (!validContact(publisher?.[field])) issues.push('A valid public ' + field + ' must be configured.');
  }
  if (!present(manifest?.terms_version) || !present(manifest?.privacy_version)
    || /draft/i.test(manifest.terms_version) || /draft/i.test(manifest.privacy_version)
    || typeof terms !== 'string' || typeof privacy !== 'string'
    || terms.toLowerCase().includes('not approved for distribution') || privacy.toLowerCase().includes('factual draft')) issues.push('Documents are missing versions or still drafts.');
  return { ready: issues.length === 0, issues, note: 'Passing these checks verifies release inputs, not legal compliance, mailbox delivery or immunity from claims.' };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const manifest = JSON.parse(readFileSync(path.join(root, 'legal/manifest.json'), 'utf8'));
  const result = checkLegal(manifest, readFileSync(path.join(root, 'legal/TERMS.md'), 'utf8'),
    readFileSync(path.join(root, 'legal/PRIVACY.md'), 'utf8'));
  console.log(JSON.stringify(result, null, 2));
  if (process.argv.includes('--release') && !result.ready) process.exitCode = 1;
}
