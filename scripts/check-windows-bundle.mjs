// Guard for what the Windows installer ships (runs in beforeBundleCommand).
//
// 1. Every shipped exe/DLL may import only: Windows inbox DLLs, DLLs the installer puts next to
//    chirp.exe, or a driver-provided DLL the installer checks for (vulkan-1.dll). A dev machine
//    has the VC++ Redistributable in System32, so "the file exists here" is not enough: runtime
//    DLLs from a redistributable never count as inbox.
// 2. A globbed resource (the llama backends) must not match two different builds. Stale
//    `llama-cpp-sys-2-*` build dirs would otherwise ship backends that do not match ggml-base.dll.
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const tauriDir = path.join(root, 'src-tauri');
const MAIN_EXE = 'target/release/chirp.exe';

/** Imports that a clean Windows has only if some installer put them there. */
export const REDISTRIBUTABLE = /^(msvcp\d|msvcr\d|vcruntime\d|vcomp\d|vccorlib\d|concrt\d|mfc\d|ucrtbased|vcamp\d)/i;
/** Present only with a GPU driver; `src-tauri/windows/installer.nsi` checks for it before installing. */
export const DRIVER_PROVIDED = new Set(['vulkan-1.dll']);

/** Names of the DLLs a PE file imports, load-time and delay-loaded. */
export function peImports(buf) {
  if (buf.length < 64 || buf.readUInt16LE(0) !== 0x5a4d) throw new Error('not an MZ image');
  const pe = buf.readUInt32LE(0x3c);
  if (buf.readUInt32LE(pe) !== 0x00004550) throw new Error('missing PE signature');
  const sectionCount = buf.readUInt16LE(pe + 6);
  const optSize = buf.readUInt16LE(pe + 20);
  const opt = pe + 24;
  const magic = buf.readUInt16LE(opt);
  const plus = magic === 0x20b;
  if (!plus && magic !== 0x10b) throw new Error(`unknown optional header magic ${magic}`);
  const imageBase = plus ? Number(buf.readBigUInt64LE(opt + 24)) : buf.readUInt32LE(opt + 28);
  const dirCount = buf.readUInt32LE(opt + (plus ? 108 : 92));
  const dirs = opt + (plus ? 112 : 96);
  const dir = i => (i < dirCount ? { rva: buf.readUInt32LE(dirs + i * 8), size: buf.readUInt32LE(dirs + i * 8 + 4) } : { rva: 0, size: 0 });
  const sections = [];
  for (let i = 0, s = opt + optSize; i < sectionCount; i++, s += 40) {
    sections.push({ va: buf.readUInt32LE(s + 12), vsize: buf.readUInt32LE(s + 8), raw: buf.readUInt32LE(s + 20), rawSize: buf.readUInt32LE(s + 16) });
  }
  const offset = rva => {
    const s = sections.find(x => rva >= x.va && rva < x.va + Math.max(x.vsize, x.rawSize));
    if (!s) throw new Error(`RVA 0x${rva.toString(16)} is outside every section`);
    return rva - s.va + s.raw;
  };
  const cstr = rva => { const at = offset(rva); return buf.toString('latin1', at, buf.indexOf(0, at)); };
  const names = [];
  const imports = dir(1);
  if (imports.rva) {
    for (let d = offset(imports.rva); ; d += 20) {
      const name = buf.readUInt32LE(d + 12);
      if (!name && !buf.readUInt32LE(d + 16)) break;
      names.push(cstr(name));
    }
  }
  const delay = dir(13);
  if (delay.rva) {
    for (let d = offset(delay.rva); ; d += 32) {
      const attrs = buf.readUInt32LE(d);
      const name = buf.readUInt32LE(d + 4);
      if (!name) break;
      names.push(cstr(attrs & 1 ? name : name - imageBase));
    }
  }
  return names;
}

/** Why `dll` is allowed for a shipped binary, or null when a clean Windows would not have it. */
export function importSource(dll, shippedAtRoot, systemDir) {
  const lower = dll.toLowerCase();
  if (shippedAtRoot.has(lower)) return 'shipped';
  if (/^(api|ext)-ms-/.test(lower)) return 'api-set';
  if (DRIVER_PROVIDED.has(lower)) return 'driver';
  if (REDISTRIBUTABLE.test(lower)) return null;
  return existsSync(path.join(systemDir, dll)) ? 'inbox' : null;
}

/** Expands `*` in path segments, relative to `base`. Returns [] when nothing matches. */
export function expand(base, pattern) {
  let current = [base];
  for (const segment of pattern.split('/')) {
    if (!segment.includes('*')) { current = current.map(p => path.join(p, segment)).filter(existsSync); continue; }
    const re = new RegExp('^' + segment.split('*').map(s => s.replace(/[.+?^${}()|[\]\\]/g, '\\$&')).join('.*') + '$', 'i');
    current = current.flatMap(p => (existsSync(p) && statSync(p).isDirectory() ? readdirSync(p).filter(n => re.test(n)).map(n => path.join(p, n)) : []));
  }
  return current;
}

const sameBytes = (a, b) => existsSync(a) && existsSync(b) && readFileSync(a).equals(readFileSync(b));

/**
 * The llama backends must come from the same llama-cpp-sys-2 build as the ggml-base.dll that
 * ships at the root. llama-cpp-sys-2 hard-links its DLLs into target/release only when they
 * are missing, so after an upgrade the root copies can be stale; and `cargo build` and
 * `cargo test` keep separate build dirs, so the glob can match two of them.
 */
export function pickBackendBuild(tauriDir, files, issues, warnings) {
  const groups = new Map();
  for (const file of files) {
    const dir = path.dirname(file);
    groups.set(dir, [...(groups.get(dir) ?? []), file]);
  }
  const rootBase = path.join(tauriDir, 'target/release/ggml-base.dll');
  const matching = [...groups.keys()].filter(dir => sameBytes(path.join(path.dirname(dir), 'bin', 'ggml-base.dll'), rootBase));
  if (matching.length !== 1) {
    issues.push(matching.length
      ? `several llama builds match target/release/ggml-base.dll: ${matching.join(', ')}`
      : 'target/release/ggml-base.dll comes from none of the current llama-cpp-sys-2 builds. Delete '
        + 'target/release/{ggml,ggml-base,llama,llama-common}.dll and rebuild so they are linked again.');
    return [];
  }
  if (groups.size > 1) {
    warnings.push(`${groups.size} llama builds match the backends glob. The one that matches the root DLLs is `
      + `${matching[0]}; scripts/package-local.ps1 pins it. A plain tauri bundle could take another.`);
  }
  return groups.get(matching[0]);
}

export function checkBundle({ tauriDir, resources, systemDir }) {
  const issues = [];
  const warnings = [];
  const shipped = []; // { file, dest }
  for (const [source, dest] of Object.entries(resources)) {
    if (!/\.(dll|exe)$/i.test(source)) continue;
    let files = source.includes('*') ? expand(tauriDir, source) : [path.join(tauriDir, source)].filter(existsSync);
    if (!files.length) { issues.push(`resource ${source} matches no file`); continue; }
    if (/llama-cpp-sys-2-\*\/out\/backends\//.test(source)) files = pickBackendBuild(tauriDir, files, issues, warnings);
    const byName = new Map();
    const conflicts = new Set();
    for (const file of files) {
      const name = path.basename(file).toLowerCase();
      const bytes = readFileSync(file);
      const seen = byName.get(name);
      if (seen && !seen.bytes.equals(bytes)) conflicts.add(path.dirname(seen.file)).add(path.dirname(file));
      if (!seen) byName.set(name, { file, bytes });
    }
    if (conflicts.size) {
      issues.push(`resource ${source} matches different builds of the same files in ${[...conflicts].join(', ')}. `
        + 'Delete the stale build dir: the one whose out/bin/ggml-base.dll differs from target/release/ggml-base.dll.');
    }
    for (const { file } of byName.values()) {
      const target = dest.endsWith('/') ? dest + path.basename(file) : dest;
      shipped.push({ file, dest: target.replace(/\\/g, '/') });
    }
  }
  const main = path.join(tauriDir, MAIN_EXE);
  if (!existsSync(main)) issues.push(`${MAIN_EXE} is missing`);
  else shipped.unshift({ file: main, dest: path.basename(main) });
  const atRoot = new Set(shipped.filter(s => !s.dest.includes('/')).map(s => path.basename(s.dest).toLowerCase()));
  const report = [];
  for (const { file, dest } of shipped) {
    let imports;
    try { imports = peImports(readFileSync(file)); } catch (e) { issues.push(`${dest}: ${e.message}`); continue; }
    const missing = imports.filter(dll => !importSource(dll, atRoot, systemDir));
    for (const dll of missing) issues.push(`${dest} imports ${dll}, which a clean Windows does not have and the installer does not ship`);
    report.push({ dest, imports: imports.length, driver: imports.filter(d => DRIVER_PROVIDED.has(d.toLowerCase())) });
  }
  return { ok: issues.length === 0, issues, warnings, shipped: report };
}

function readResources() {
  const read = name => JSON.parse(readFileSync(path.join(tauriDir, name), 'utf8'));
  return { ...(read('tauri.conf.json').bundle?.resources ?? {}), ...(read('tauri.windows.conf.json').bundle?.resources ?? {}) };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.platform !== 'win32') process.exit(0);
  const systemDir = path.join(process.env.SystemRoot ?? 'C:\\Windows', 'System32');
  const result = checkBundle({ tauriDir, resources: readResources(), systemDir });
  for (const warning of result.warnings) console.warn(`check-windows-bundle: warning: ${warning}`);
  if (!result.ok) {
    for (const issue of result.issues) console.error(`check-windows-bundle: ${issue}`);
    process.exit(1);
  }
  console.log(`check-windows-bundle: ${result.shipped.length} binaries, imports resolved`);
}
