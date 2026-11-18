// Guard for the forked NSIS template (src-tauri/windows/installer.nsi).
//
// The fork starts from the stock template embedded in @tauri-apps/cli 2.11.2.
// If the CLI changes, the stock template may change with it (new flags, fixed
// bugs, renamed variables) and the fork would silently drift. This check runs
// before bundling and fails until the fork is re-synced with the new upstream
// template and the constants below are updated.
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const BASE_CLI_VERSION = '2.11.2';
// SHA-256 of the stock template bytes as embedded in the 2.11.2 Windows CLI.
const BASE_TEMPLATE_SHA256 = '7e93df962d3d2f4539a9f9fd0f1cecc065cfa5275c35c139ccb958325a060fae';

if (process.platform !== 'win32') {
  // The NSIS template only matters for Windows bundles.
  process.exit(0);
}

const require = createRequire(import.meta.url);
const fail = (message) => {
  console.error(`check-nsis-template: ${message}`);
  console.error('Re-sync src-tauri/windows/installer.nsi with the new stock template, then update scripts/check-nsis-template.mjs.');
  process.exit(1);
};

const cliVersion = require('@tauri-apps/cli/package.json').version;
if (cliVersion !== BASE_CLI_VERSION) {
  fail(`@tauri-apps/cli is ${cliVersion}, the fork is based on ${BASE_CLI_VERSION}.`);
}

let binary;
try {
  binary = readFileSync(require.resolve('@tauri-apps/cli-win32-x64-msvc/cli.win32-x64-msvc.node'));
} catch {
  fail('cannot read the Windows CLI binary to compare its stock template.');
}

const start = binary.indexOf(Buffer.from('Unicode true\r\nManifestDPIAware true'));
if (start < 0) fail('stock template not found in the CLI binary.');
// The template is plain text; it ends at the first control byte other than tab/CR/LF.
let end = start;
while (end < binary.length) {
  const byte = binary[end];
  if (byte < 0x09 || (byte > 0x0d && byte < 0x20)) break;
  end += 1;
}
const hash = createHash('sha256').update(binary.subarray(start, end)).digest('hex');
if (hash !== BASE_TEMPLATE_SHA256) {
  fail(`stock template changed (sha256 ${hash}).`);
}
console.log(`check-nsis-template: stock template matches ${BASE_CLI_VERSION}.`);
