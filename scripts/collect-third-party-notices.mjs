/** Collect exact installed-package license/NOTICE files without installing anything.
 * node scripts/collect-third-party-notices.mjs
 * --refresh-models fetches official model license/terms snapshots (explicit network).
 * --recover-notices recovers target-runtime gaps at exact upstream crate commits.
 * Every run regenerates offline target-graph dispositions and verifies cached recoveries.
 * The all-target Cargo resolution and npm lock include build/dev/optional packages;
 * this inventory is deliberately NOT a claim that every entry ships in the app.
 */
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
const root = fileURLToPath(new URL('..', import.meta.url));
const out = path.join(root, 'legal', 'third-party');
const slash = value => value.replaceAll('\\', '/');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const readJson = file => JSON.parse(fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, ''));
const safe = value => value.replace(/[^a-zA-Z0-9._-]/g, '_');
const relative = file => slash(path.relative(out, file));
function write(file, content) { fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, content); }
function json(file, value) { write(file, JSON.stringify(value, null, 2) + '\n'); }
const missing = [];
function collect(sourceRoot, destination, declaredFile) {
  const files = [];
  const visited = new Set();
  function copy(source, sourceRelative) {
    const full = path.resolve(source);
    if (visited.has(full)) return;
    visited.add(full);
    if (!fs.statSync(full).isFile()) return;
    if (fs.statSync(full).size > 2 * 1024 * 1024) { missing.push({ source: slash(sourceRelative), reason: 'oversize_notice_file' }); return; }
    const target = path.join(destination, sourceRelative);
    if (!path.resolve(target).startsWith(path.resolve(destination) + path.sep)) throw new Error('Unsafe notice path');
    const bytes = fs.readFileSync(full);
    write(target, bytes);
    files.push({ source_path: slash(sourceRelative), copied_path: relative(target), sha256: hash(bytes), bytes: bytes.length, nested_component: sourceRelative.includes(path.sep) });
  }
  function visit(dir, prefix = '', depth = 0, licenseDirectory = false) {
    if (depth > 12) return;
    for (const entry of fs.readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      if (entry.isSymbolicLink()) continue;
      const name = entry.name;
      const sourceRelative = path.join(prefix, name);
      if (entry.isDirectory()) {
        if (['.git', 'node_modules', 'target', '.next', '.cache'].includes(name)) continue;
        visit(path.join(dir, name), sourceRelative, depth + 1, licenseDirectory || /^(licen[cs]es?|notices?|copyrights?)$/i.test(name));
      } else if (entry.isFile() && (/^(licen[cs]e|notice|copying|copyright|unlicense)([._-].*)?$/i.test(name) || (licenseDirectory && /\.(txt|md|rst|html)$/i.test(name)))) copy(path.join(dir, name), sourceRelative);
    }
  }
  visit(sourceRoot);
  if (declaredFile) {
    const absolute = path.resolve(sourceRoot, declaredFile);
    const rel = path.relative(sourceRoot, absolute);
    if (!rel.startsWith('..') && !path.isAbsolute(rel) && fs.existsSync(absolute)) copy(absolute, rel);
  }
  return files.sort((a, b) => a.source_path.localeCompare(b.source_path));
}
fs.mkdirSync(out, { recursive: true });
const cargoRun = spawnSync('cargo', ['metadata', '--locked', '--offline', '--format-version', '1', '--manifest-path', 'src-tauri/Cargo.toml'], { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, windowsHide: true });
if (cargoRun.status !== 0) throw new Error(`Offline Cargo metadata failed: ${cargoRun.stderr}`);
const metadata = JSON.parse(cargoRun.stdout);
const firstParty = new Set(metadata.workspace_members);
const cargo = metadata.packages.filter(pkg => !firstParty.has(pkg.id)).map(pkg => {
  const sourceRoot = path.dirname(pkg.manifest_path);
  const files = collect(sourceRoot, path.join(out, 'cargo', `${safe(pkg.name)}-${pkg.version}`), pkg.license_file);
  const record = { ecosystem: 'cargo', name: pkg.name, version: pkg.version, declared_license: pkg.license, source: pkg.source, source_package_url: pkg.source?.startsWith('registry+') ? `https://crates.io/crates/${pkg.name}/${pkg.version}` : pkg.repository, repository: pkg.repository, source_present: true, scope: 'all-target resolved graph; includes build, proc-macro, dev and target-specific dependencies; shipment not established', notices: files };
  if (!files.length) missing.push({ ecosystem: 'cargo', name: pkg.name, version: pkg.version, reason: 'no_license_notice_file_in_installed_source', declared_license: pkg.license });
  return record;
}).sort((a, b) => `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`));
const lock = readJson(path.join(root, 'package-lock.json'));
const npm = Object.entries(lock.packages).filter(([key]) => key).map(([key, pkg]) => {
  const sourceRoot = path.join(root, key);
  const name = pkg.name || key.split('node_modules/').at(-1);
  const manifest = path.join(sourceRoot, 'package.json');
  const installed = fs.existsSync(manifest) ? readJson(manifest) : null;
  const matching = installed?.version === pkg.version;
  const files = matching ? collect(sourceRoot, path.join(out, 'npm', `${safe(name)}-${pkg.version}`)) : [];
  const record = { ecosystem: 'npm', name, version: pkg.version, declared_license: pkg.license || installed?.license || null, source_package_url: `https://www.npmjs.com/package/${name}/v/${pkg.version}`, resolved: pkg.resolved, integrity: pkg.integrity, dev: !!pkg.dev, optional: !!pkg.optional, source_present: !!installed, installed_version_matches_lock: matching, scope: pkg.dev ? 'development/build dependency; shipment not established' : 'production-declared dependency; bundle inclusion not independently established', notices: files };
  if (!files.length) missing.push({ ecosystem: 'npm', name, version: pkg.version, reason: !installed ? 'source_not_installed_on_this_platform' : !matching ? 'installed_version_differs_from_lock' : 'no_license_notice_file_in_installed_source', declared_license: record.declared_license });
  return record;
}).sort((a, b) => `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`));
json(path.join(out, 'inventory.json'), {
  format: 1,
  scope: 'Conservative dependency notice inventory from locked local source packages. Not an SBOM of a proven installer, not a legal-compliance certificate. Nested notices may cover optional/example code not shipped. No license expression is silently simplified.',
  lockfiles: { cargo_sha256: hash(fs.readFileSync(path.join(root, 'src-tauri/Cargo.lock'))), npm_sha256: hash(fs.readFileSync(path.join(root, 'package-lock.json'))) },
  first_party: metadata.packages.filter(pkg => firstParty.has(pkg.id)).map(pkg => ({ name: pkg.name, version: pkg.version, declared_license: pkg.license })),
  cargo, npm,
});
json(path.join(out, 'missing-notices.json'), missing);
const modelSources = [
  { name: 'Whisper weights', file: 'whisper/LICENSE', url: 'https://raw.githubusercontent.com/openai/whisper/main/LICENSE', type: 'text', license: 'MIT' },
  { name: 'Whisper.cpp runtime', file: 'whisper-cpp/LICENSE', url: 'https://raw.githubusercontent.com/ggml-org/whisper.cpp/master/LICENSE', type: 'text', license: 'MIT' },
  { name: 'Silero VAD', file: 'silero-vad/LICENSE', url: 'https://raw.githubusercontent.com/snakers4/silero-vad/master/LICENSE', type: 'text', license: 'MIT' },
  { name: 'Qwen3-4B-GGUF', file: 'qwen3-4b/LICENSE', url: 'https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/main/LICENSE', type: 'text', license: 'Apache-2.0' },
  { name: 'Gemma 3 terms', file: 'gemma-3/TERMS.html', url: 'https://ai.google.dev/gemma/terms', type: 'html', license: 'Gemma Terms of Use; not Apache-2.0' },
  { name: 'Gemma prohibited use policy', file: 'gemma-3/PROHIBITED-USE-POLICY.html', url: 'https://ai.google.dev/gemma/prohibited_use_policy', type: 'html', license: 'Incorporated into Gemma Terms of Use' },
];
if (process.argv.includes('--refresh-models')) {
  const receipts = [];
  for (const source of modelSources) {
    const response = await fetch(source.url, { signal: AbortSignal.timeout(30000) });
    if (!response.ok) throw new Error(`Model terms fetch failed (${response.status}): ${source.url}`);
    const bytes = Buffer.from(await response.arrayBuffer());
    const content = bytes.toString('utf8');
    if (source.type === 'text' && /<!doctype html|<html[ >]/i.test(content)) throw new Error(`Expected license text, received HTML: ${source.url}`);
    if (bytes.length < 200 || bytes.length > 2 * 1024 * 1024) throw new Error(`Unexpected terms size: ${source.url}`);
    write(path.join(out, 'models', source.file), bytes);
    receipts.push({ ...source, sha256: hash(bytes), bytes: bytes.length, retrieved_at: new Date().toISOString(), resolved_url: response.url, scope: 'Official-source snapshot; moving upstream URL. Match terms/provenance to exact distributed model revision before release.' });
  }
  write(path.join(out, 'models/gemma-3/NOTICE'), 'Gemma is provided under and subject to the Gemma Terms of Use found at ai.google.dev/gemma/terms\n');
  json(path.join(out, 'models/sources.json'), receipts);
}
// Keep canonical HTML snapshots byte-for-byte, plus readable offline text that
// never executes upstream page scripts. The text is explicitly a derived view.
for (const source of modelSources.filter(item => item.type === 'html')) {
  const original = path.join(out, 'models', source.file);
  if (!fs.existsSync(original)) continue;
  const html = fs.readFileSync(original, 'utf8');
  const article = html.match(/<div class="devsite-article-body[^\"]*">([\s\S]*?)<devsite-thumb-rating/);
  if (!article) throw new Error(`Review terms extraction after upstream HTML change: ${source.url}`);
  const entities = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: ' ', ndash: '–', mdash: '—', rsquo: '’', lsquo: '‘', rdquo: '”', ldquo: '“' };
  const text = article[1]
    .replace(/<(script|style)\b[^>]*>[\s\S]*?<\/\1>/gi, '')
    .replace(/<a\b[^>]*href="([^"]+)"[^>]*>([\s\S]*?)<\/a>/gi, (_match, href, label) => `${label} (${new URL(href, source.url).href})`)
    .replace(/<(?:p|h[1-6]|li|ul|ol|aside|br)\b[^>]*>/gi, '\n')
    .replace(/<\/(?:p|h[1-6]|li|ul|ol|aside)>/gi, '\n')
    .replace(/<[^>]+>/g, '')
    .replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (match, key) => key[0] === '#' ? String.fromCodePoint(key[1].toLowerCase() === 'x' ? parseInt(key.slice(2), 16) : Number(key.slice(1))) : entities[key] ?? match)
    .split('\n').map(line => line.trim()).join('\n').replace(/\n{3,}/g, '\n\n').trim();
  if (text.length < 1000) throw new Error(`Terms extraction unexpectedly short: ${source.url}`);
  write(original.replace(/\.html$/, '.txt'), `Readable text extracted from the accompanying unmodified HTML snapshot.\nSource: ${source.url}\nSee models/sources.json for retrieval time and canonical HTML hash.\n\n${source.name}\n\n${text}\n`);
}
const count = packages => packages.reduce((sum, pkg) => sum + pkg.notices.length, 0);
write(path.join(out, 'README.md'), `# Third-party notices\n\nGenerated by \`node scripts/collect-third-party-notices.mjs\` using locked, offline Cargo metadata and installed npm sources. No package installation or build is performed. \`--refresh-models\` explicitly refreshes official model license/terms snapshots; ordinary runs do not contact the network.\n\nInventory: ${cargo.length} Cargo packages and ${npm.length} npm lock entries. Exact license/notice copies: ${count(cargo) + count(npm)}. Original source-package notice gaps: ${missing.length}; see \`missing-notices.json\`.\n\nThis conservative all-target/development inventory is **not proof of the components in any installer**. A declared production dependency may be tree-shaken; build/proc-macro and other-platform sources may never ship. Nested vendor/example licenses are preserved without claiming every nested component is distributed. Preserve each package's selected license obligations and exact copyright text. The scanner does not extract source-file-only license headers, infer missing notices, or certify compliance.\n\n\`inventory.json\` records source package URLs, versions, original license expressions, copied relative paths, file hashes and lockfile hashes. \`models/sources.json\` records official model snapshot provenance. Existing snapshots are not deleted automatically; use the inventory as the current file list.\n\nBefore release, reconcile the actual Windows/macOS artifacts (including C/C++ vendor code, DLLs, WebView2, OpenMP/Vulkan/Metal redistributables, model files and artwork) with this inventory. MPL entries need an actual shipped-source assessment. Gemma 3 needs its separate terms, restrictions and Notice; it must not be labeled Apache-2.0. No publisher identity or application license is inferred by this collector.\n`);
console.log(JSON.stringify({ cargo_packages: cargo.length, npm_lock_entries: npm.length, copied_notices: count(cargo) + count(npm), raw_source_gaps: missing.length, output: slash(path.relative(root, out)) }, null, 2));

// Reconcile package-source gaps against target graphs without claiming that a
// dependency graph proves the final linker's/installer's exact contents.
async function reconcileNotices() {
  const base = path.join(out, 'reconciliation');
  const packageKey = pkg => `${pkg.name}@${pkg.version}`;
  const targets = [ ['windows_x64', 'x86_64-pc-windows-msvc'], ['macos_arm64', 'aarch64-apple-darwin'] ];
  const graphs = {};
  const evidence = [];
  const parseTree = text => new Set(text.split(/\r?\n/).map(line => line.match(/^([^\s]+) v([^\s]+)/)).filter(Boolean).map(match => `${match[1]}@${match[2]}`));
  for (const [name, target] of targets) {
    graphs[name] = {};
    for (const [kind, edges] of [['runtime_candidates', 'normal,no-proc-macro'], ['production_with_build', 'normal,build'], ['including_dev', 'normal,build,dev']]) {
      const args = ['tree', '--locked', '--offline', '--manifest-path', 'src-tauri/Cargo.toml', '--target', target, '--edges', edges, '--prefix', 'none', '--format', '{p}'];
      const run = spawnSync('cargo', args, { cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024, windowsHide: true });
      if (run.status !== 0) throw new Error(`Target dependency classification failed: ${run.stderr}`);
      const text = run.stdout.replaceAll(root, '<workspace>');
      const file = path.join(base, 'evidence', `cargo-${name}-${kind}.txt`);
      write(file, text);
      evidence.push({ command: ['cargo', ...args], file: relative(file), sha256: hash(Buffer.from(text)), exit_code: run.status });
      graphs[name][kind] = parseTree(run.stdout);
    }
  }
  const npmCli = [process.env.npm_execpath, path.join(path.dirname(process.execPath), 'node_modules/npm/bin/npm-cli.js'), path.resolve(path.dirname(process.execPath), '../lib/node_modules/npm/bin/npm-cli.js')].find(candidate => candidate && fs.existsSync(candidate));
  if (!npmCli) throw new Error('Could not locate npm CLI; run through npm or provide npm_execpath for offline production-tree classification');
  const npmArgs = ['ls', '--omit=dev', '--all', '--json', '--offline'];
  const npmRun = spawnSync(process.execPath, [npmCli, ...npmArgs], { cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024, windowsHide: true });
  const npmTree = JSON.parse(npmRun.stdout);
  const productionNpm = new Set();
  const addNpm = node => { for (const [name, value] of Object.entries(node.dependencies || {})) { productionNpm.add(`${name}@${value.version}`); addNpm(value); } };
  addNpm(npmTree);
  // Omit absolute local paths from portable evidence; retain version graph and errors.
  const npmEvidence = JSON.stringify(npmTree, (key, value) => key === 'path' ? '<workspace>' : value, 2) + '\n';
  write(path.join(base, 'evidence/npm-production-tree.json'), npmEvidence);
  evidence.push({ command: ['npm', ...npmArgs], file: 'reconciliation/evidence/npm-production-tree.json', sha256: hash(Buffer.from(npmEvidence)), exit_code: npmRun.status });
  const recoveryFile = path.join(base, 'recoveries.json');
  const priorRecoveries = fs.existsSync(recoveryFile) ? readJson(recoveryFile) : [];
  const recoveries = new Map(priorRecoveries.map(item => [packageKey(item), item]));
  const dispositions = missing.map(gap => {
    const key = packageKey(gap);
    if (gap.ecosystem === 'npm') {
      const pkg = npm.find(item => packageKey(item) === key);
      return { ...gap, classification: productionNpm.has(key) ? 'installed_production_dependency_candidate' : pkg?.dev && pkg?.optional && !pkg?.source_present ? 'optional_build_dependency_not_installed_on_this_host' : pkg?.dev ? 'build_tool_dependency' : 'not_in_installed_production_tree', source_package_url: pkg?.source_package_url, declared_dev: pkg?.dev, declared_optional: pkg?.optional, recovery: null, action: 'No unused platform package downloaded. Check toolchain redistribution only if the tool binary/package is actually included in the release.' };
    }
    const pkg = metadata.packages.find(item => packageKey(item) === key);
    const byTarget = Object.fromEntries(targets.map(([name]) => [name,
      graphs[name].runtime_candidates.has(key) ? 'runtime_dependency_candidate' : graphs[name].production_with_build.has(key) ? 'build_or_proc_macro_only' : graphs[name].including_dev.has(key) ? 'dev_only' : 'not_in_this_target_graph']));
    const sourceRoot = path.dirname(pkg.manifest_path);
    const vcsPath = path.join(sourceRoot, '.cargo_vcs_info.json');
    const vcs = fs.existsSync(vcsPath) ? readJson(vcsPath) : null;
    return { ...gap, classification: Object.values(byTarget).includes('runtime_dependency_candidate') ? 'runtime_candidate_needs_notice_review' : Object.values(byTarget).includes('build_or_proc_macro_only') ? 'build_or_proc_macro_only' : Object.values(byTarget).includes('dev_only') ? 'dev_only' : 'other_target_or_inactive_dependency', targets: byTarget, source_package_url: `https://crates.io/crates/${pkg.name}/${pkg.version}`, repository: pkg.repository, cargo_vcs: vcs, cargo_vcs_sha256: vcs ? hash(fs.readFileSync(vcsPath)) : null, recovery: recoveries.get(key) || null, action: 'Review exact upstream text and final distributed artifact; graph reachability is not proof of binary inclusion.' };
  });
  if (process.argv.includes('--recover-notices')) {
    const responseCache = new Map();
    async function get(url) {
      if (responseCache.has(url)) return responseCache.get(url);
      const promise = (async () => {
        const response = await fetch(url, { headers: { 'User-Agent': 'Lectus-third-party-notices-audit' }, signal: AbortSignal.timeout(25000) });
        if (!response.ok) throw new Error(`HTTP ${response.status}: ${url}`);
        const bytes = Buffer.from(await response.arrayBuffer());
        if (bytes.length > 3 * 1024 * 1024) throw new Error(`Unexpectedly large license response: ${url}`);
        return { bytes, resolved_url: response.url };
      })();
      responseCache.set(url, promise);
      return promise;
    }
    for (const disposition of dispositions.filter(item => item.ecosystem === 'cargo' && item.classification === 'runtime_candidate_needs_notice_review')) {
      const key = packageKey(disposition);
      if (recoveries.get(key)?.notices?.length) continue;
      const pkg = metadata.packages.find(item => packageKey(item) === key);
      let commit = disposition.cargo_vcs?.git?.sha1;
      const repository = pkg.repository?.match(/^https?:\/\/github\.com\/([^/]+)\/([^/#]+)/i);
      if (!repository || disposition.cargo_vcs?.git?.dirty) {
        recoveries.set(key, { name: pkg.name, version: pkg.version, status: 'manual_exact_release_provenance_needed', reason: !repository ? 'repository unsupported for automatic pinned lookup' : 'crate vcs metadata records dirty source', notices: [] });
        continue;
      }
      const repo = `${repository[1]}/${repository[2].replace(/\.git$/, '')}`;
      const sourcePath = (disposition.cargo_vcs?.path_in_vcs || '').replace(/^\/+|\/+$/g, '');
      const record = { name: pkg.name, version: pkg.version, status: 'manual_review_needed', commit, repository: `https://github.com/${repo}`, path_in_vcs: sourcePath, provenance: 'Commit from exact installed crate .cargo_vcs_info.json; upstream files fetched at that immutable commit', notices: [], errors: [] };
      try {
        if (!commit) {
          const tagsUrl = `https://api.github.com/repos/${repo}/tags?per_page=100`;
          const tags = JSON.parse((await get(tagsUrl)).bytes);
          const tag = tags.find(item => [pkg.version, `v${pkg.version}`, `${pkg.name}-${pkg.version}`].includes(item.name));
          if (!tag) throw new Error('No exact release tag found; do not substitute current main');
          commit = tag.commit.sha; record.commit = commit; record.release_tag = tag.name; record.release_tags_url = tagsUrl;
          record.provenance = 'Authoritative exact-version tag plus all packaged Rust source blobs matched to the upstream tree';
        }
        const treeUrl = `https://api.github.com/repos/${repo}/git/trees/${commit}?recursive=1`;
        const { bytes } = await get(treeUrl);
        const tree = JSON.parse(bytes);
        if (tree.truncated) throw new Error('Upstream tree truncated; automatic scope incomplete');
        record.tree_url = treeUrl;
        record.tree_sha256 = hash(bytes);
        const rootSource = path.dirname(pkg.manifest_path);
        if (record.release_tag) {
          const sourceFiles = [];
          function findRust(dir, prefix = '') {
            for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
              const relative = path.posix.join(prefix, entry.name);
              if (entry.isDirectory() && !['target', '.git'].includes(entry.name)) findRust(path.join(dir, entry.name), relative);
              else if (entry.isFile() && entry.name.endsWith('.rs')) sourceFiles.push(relative);
            }
          }
          findRust(rootSource);
          if (!sourceFiles.length) throw new Error('No packaged Rust source files to bind release tag to crate');
          record.matched_source_blobs = sourceFiles.map(relative => {
            const sourceBytes = fs.readFileSync(path.join(rootSource, relative));
            const blob = createHash('sha1').update(`blob ${sourceBytes.length}\0`).update(sourceBytes).digest('hex');
            const upstreamPath = path.posix.join(sourcePath, relative);
            if (tree.tree.find(item => item.path === upstreamPath && item.type === 'blob')?.sha !== blob) throw new Error(`Release tag differs from packaged source: ${relative}`);
            return { path: relative, git_blob_sha1: blob };
          });
        }
        const scopes = new Set(['']);
        let cursor = sourcePath;
        while (cursor) { scopes.add(cursor); const parent = path.posix.dirname(cursor); cursor = parent === '.' ? '' : parent; }
        // A workspace/root license can govern a member crate. Do not select
        // unrelated sibling packages merely because they contain LICENSE.
        const paths = tree.tree.filter(item => item.type === 'blob' && /^(licen[cs]e|notice|copying|copyright|unlicense)([._-].*)?$/i.test(path.posix.basename(item.path)) && scopes.has(path.posix.dirname(item.path) === '.' ? '' : path.posix.dirname(item.path))).map(item => item.path);
        for (const source of paths.sort()) {
          const url = `https://raw.githubusercontent.com/${repo}/${commit}/${source}`;
          const response = await get(url);
          if (response.bytes.length < 30 || /<!doctype html|<html[ >]/i.test(response.bytes.toString('utf8').slice(0, 500))) throw new Error(`Not a license text: ${url}`);
          const file = path.join(base, 'recovered', `${safe(pkg.name)}-${pkg.version}`, source);
          write(file, response.bytes);
          record.notices.push({ source_path: source, source_url: url, resolved_url: response.resolved_url, copied_path: relative(file), sha256: hash(response.bytes), bytes: response.bytes.length, kind: 'upstream_notice_at_crate_commit' });
        }
        record.status = record.notices.length ? 'exact_commit_notice_text_recovered_review_required' : 'no_ancestor_notice_at_exact_commit';
        if (!record.notices.length) {
          // Preserve the real declaration; never fill a missing MIT copyright
          // notice using guessed author/year boilerplate.
          const source = path.posix.join(sourcePath, 'Cargo.toml');
          const url = `https://raw.githubusercontent.com/${repo}/${commit}/${source}`;
          const response = await get(url);
          const file = path.join(base, 'recovered', `${safe(pkg.name)}-${pkg.version}`, 'DECLARATION-Cargo.toml');
          write(file, response.bytes);
          record.declaration_evidence = { source_url: url, copied_path: relative(file), sha256: hash(response.bytes), bytes: response.bytes.length, kind: 'license_metadata_declaration_not_full_license_notice' };
        }
      } catch (error) { record.errors.push(error.message); }
      recoveries.set(key, record);
      json(recoveryFile, [...recoveries.values()].sort((a, b) => packageKey(a).localeCompare(packageKey(b))));
      console.log(`Notices ${key}: ${record.status} (${record.notices.length})`);
    }
  }
  for (const recovery of recoveries.values()) {
    for (const notice of recovery.notices || []) {
      const text = fs.readFileSync(path.join(out, notice.copied_path), 'utf8');
      notice.content_kind = recovery.repository === 'https://github.com/madsmtm/objc2' && notice.source_path === 'LICENSE.md'
        ? 'upstream_license_declaration_and_apple_sdk_caveat_not_full_license_text'
        : /Permission is hereby granted|TERMS AND CONDITIONS FOR USE, REPRODUCTION|Redistribution and use in source and binary|Boost Software License/.test(text)
          ? 'full_license_text_present' : /copyright/i.test(text) ? 'copyright_notice' : 'notice_text_manual_classification';
    }
    recovery.material_scope = recovery.notices?.some(notice => notice.content_kind === 'full_license_text_present')
      ? 'full_license_text_recovered_legal_selection_and_artifact_review_still_required'
      : recovery.notices?.length ? 'licensing_declaration_only_manual_terms_and_sdk_review_required'
      : 'metadata_declaration_only_no_full_license_or_copyright_notice_recovered';
  }
  json(recoveryFile, [...recoveries.values()].sort((a, b) => packageKey(a).localeCompare(packageKey(b))));
  for (const item of dispositions) if (item.ecosystem === 'cargo') item.recovery = recoveries.get(packageKey(item)) || null;
  const summary = {
    original_source_gaps: dispositions.length,
    npm_installed_production_gaps: dispositions.filter(item => item.ecosystem === 'npm' && item.classification === 'installed_production_dependency_candidate').length,
    npm_build_only: dispositions.filter(item => item.ecosystem === 'npm' && item.classification === 'build_tool_dependency').length,
    npm_optional_build_not_installed: dispositions.filter(item => item.classification === 'optional_build_dependency_not_installed_on_this_host').length,
    cargo_runtime_candidates: dispositions.filter(item => item.ecosystem === 'cargo' && item.classification === 'runtime_candidate_needs_notice_review').length,
    cargo_runtime_notice_text_recovered: dispositions.filter(item => item.ecosystem === 'cargo' && item.classification === 'runtime_candidate_needs_notice_review' && item.recovery?.notices?.length).length,
    cargo_runtime_full_license_text: dispositions.filter(item => item.classification === 'runtime_candidate_needs_notice_review' && item.recovery?.notices?.some(notice => notice.content_kind === 'full_license_text_present')).length,
    cargo_runtime_declaration_only: dispositions.filter(item => item.classification === 'runtime_candidate_needs_notice_review' && item.recovery?.notices?.length && !item.recovery.notices.some(notice => notice.content_kind === 'full_license_text_present')).length,
    cargo_runtime_no_full_notice: dispositions.filter(item => item.classification === 'runtime_candidate_needs_notice_review' && !item.recovery?.notices?.length).length,
    windows_runtime_candidates: dispositions.filter(item => item.targets?.windows_x64 === 'runtime_dependency_candidate').length,
    macos_runtime_candidates: dispositions.filter(item => item.targets?.macos_arm64 === 'runtime_dependency_candidate').length,
    cargo_build_only: dispositions.filter(item => item.ecosystem === 'cargo' && item.classification === 'build_or_proc_macro_only').length,
    cargo_dev_only: dispositions.filter(item => item.ecosystem === 'cargo' && item.classification === 'dev_only').length,
    cargo_other_target_or_inactive: dispositions.filter(item => item.ecosystem === 'cargo' && item.classification === 'other_target_or_inactive_dependency').length,
  };
  json(path.join(base, 'dispositions.json'), { format: 1, scope: 'Default feature dependency graphs for x86_64-pc-windows-msvc and aarch64-apple-darwin, without building/downloading packages. Runtime candidates exclude build edges and proc-macro subtrees. Exact link/bundle inclusion and license compliance remain separate review tasks.', lockfiles: { cargo_sha256: hash(fs.readFileSync(path.join(root, 'src-tauri/Cargo.lock'))), npm_sha256: hash(fs.readFileSync(path.join(root, 'package-lock.json'))) }, evidence, summary, dispositions });
  // Verify previously recovered bytes rather than silently trusting a stale receipt.
  for (const recovery of recoveries.values()) for (const notice of recovery.notices || []) {
    if (hash(fs.readFileSync(path.join(out, notice.copied_path))) !== notice.sha256) throw new Error(`Recovered notice changed: ${notice.copied_path}`);
  }
  for (const recovery of recoveries.values()) {
    const proof = recovery.declaration_evidence;
    if (proof && hash(fs.readFileSync(path.join(out, proof.copied_path))) !== proof.sha256) throw new Error(`Declaration evidence changed: ${proof.copied_path}`);
  }
  const inventoryPath = path.join(out, 'inventory.json');
  const inventory = readJson(inventoryPath);
  inventory.notice_reconciliation = { file: 'reconciliation/dispositions.json', recoveries: 'reconciliation/recoveries.json', raw_source_gap_count: missing.length, complete_legal_review: false };
  for (const pkg of inventory.cargo) {
    const item = dispositions.find(item => item.ecosystem === 'cargo' && packageKey(item) === packageKey(pkg));
    if (item) { pkg.target_classification = item.targets; pkg.recovered_notice_files = item.recovery?.notices || []; pkg.recovered_material_scope = item.recovery?.material_scope || 'not_recovered_non_runtime_or_inactive_scope'; }
  }
  for (const pkg of inventory.npm) {
    const item = dispositions.find(item => item.ecosystem === 'npm' && packageKey(item) === packageKey(pkg));
    if (item) pkg.production_tree_classification = item.classification;
  }
  json(inventoryPath, inventory);
  const runtime = dispositions.filter(item => item.classification === 'runtime_candidate_needs_notice_review');
  const runtimeRows = runtime.map(item => `| ${item.name} ${item.version} | ${item.targets.windows_x64 === 'runtime_dependency_candidate' ? 'Yes' : 'No'} | ${item.targets.macos_arm64 === 'runtime_dependency_candidate' ? 'Yes' : 'No'} | ${item.recovery?.material_scope || 'Unresolved'} |`).join('\n');
  const report = `# Notice-source reconciliation\n\nThis is a classification of the original ${missing.length} source-package gaps, **not proof that all these packages ship and not legal approval**. Default features and the locked Windows x64 / macOS arm64 graphs were used. Runtime candidate graphs exclude build edges and proc-macro subtrees; raw graphs and the npm production tree are preserved in evidence/.\n\n| Disposition | Count |\n|---|---:|\n| npm production-tree gaps | ${summary.npm_installed_production_gaps} |\n| npm installed build tools | ${summary.npm_build_only} |\n| npm optional build packages not installed | ${summary.npm_optional_build_not_installed} |\n| Cargo runtime candidates, either target | ${summary.cargo_runtime_candidates} |\n| Cargo build/proc-macro only | ${summary.cargo_build_only} |\n| Cargo dev only | ${summary.cargo_dev_only} |\n| Cargo other target or inactive | ${summary.cargo_other_target_or_inactive} |\n\nWithin runtime candidates: ${summary.cargo_runtime_full_license_text} have exact upstream full-license text recovered; ${summary.cargo_runtime_declaration_only} have upstream licensing declarations/Apple SDK caveats only; ${summary.cargo_runtime_no_full_notice} have no full license/notice recovered. All ${summary.windows_runtime_candidates} Windows runtime source gaps have recovered material; inspect each row for scope. Unused platform packages were not downloaded.\n\n## Runtime candidate detail\n\n| Crate | Windows x64 | macOS arm64 | Material recovered / pending |\n|---|---|---|---|\n${runtimeRows}\n\n## Remaining release review\n\n1. Reconcile the actual linked/bundled artifacts with this graph, including static native code, DLLs/dylibs, SDK/runtime redistributables and model weights. A graph can over-include feature or linker-eliminated code.\n2. Decide and record applicable license alternatives and obligations. Preserve AND expressions and exact copyright text; recovered files do not themselves prove compliance.\n3. Review the objc2 LICENSE.md declarations and Apple SDK caveat. These files link standard license terms but are not complete license/copyright notices; do not silently substitute a guessed MIT header.\n4. Review block 0.1.6, malloc_buf 0.0.6 and sigchld 0.2.4. Their exact release/commit sources declare MIT but contain no ancestor LICENSE/NOTICE; corresponding Cargo.toml declarations are retained. The first two exact tags were bound to installed crates by matching every packaged Rust source blob. No copyright year or owner was invented.\n5. Build-only/other-target gaps are not dropped: retain their dispositions and review if those tools, source packages or platforms are later distributed. No publisher identity, publication approval or third_party_notices_reviewed flag was set.\n\nUse node scripts/collect-third-party-notices.mjs for reproducible offline regeneration and cached-byte verification. --recover-notices explicitly fetches primary upstream source at exact crate commits/tags; --refresh-models separately refreshes model terms. Raw source gaps remain in ../missing-notices.json so recovered evidence does not erase the original finding.\n`;
  write(path.join(base, 'README.md'), report);
  fs.appendFileSync(path.join(out, 'README.md'), '\n## Target-specific reconciliation\n\nThe original source gaps are classified in [reconciliation/README.md](reconciliation/README.md) with machine-readable dispositions and exact-commit recovered notices. Use that report to distinguish runtime candidates from build/optional/inactive entries. The number in missing-notices.json is the raw installed-source gap count, not the number of unresolved runtime release obligations.\n');
  console.log(JSON.stringify({ reconciliation: summary }, null, 2));
}
await reconcileNotices();
