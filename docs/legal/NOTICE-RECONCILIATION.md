# Notice-gap reconciliation — 2026-10-21

The original 130 entries were **source-package notice gaps**, not 130 demonstrated missing runtime licenses. Locked dependency graphs now separate their scopes. Nothing here establishes the final installer's contents or approves legal distribution.

## Result

| Original gap group | Count | Disposition |
|---|---:|---|
| npm optional build packages absent on this host | 59 | No unused platform packages downloaded. Retained for toolchain/platform review if later distributed. |
| Installed npm build tools | 4 | esbuild, Rollup and Tauri CLI native packages; none appears in the installed npm production tree. |
| Cargo build/proc-macro only | 4 | find_cuda_helper, selectors, tauri-plugin and webview2-com-macros under the tested target graphs. |
| Cargo other-target/inactive entries | 35 | Not reached in the default-feature Windows x64 or macOS arm64 graphs used here. |
| Cargo runtime candidates, either target | 28 | All inspected; exact-source materials recovered where present. |

There are **zero gaps in the installed npm production dependency tree**. Among the 28 Cargo runtime candidates, 11 occur in Windows x64 and 25 in macOS arm64 (eight overlap). These are dependency candidates, not claims that a linker retained every component.

Of those 28, **15 have full license text recovered**, **10 have upstream licensing declarations/Apple SDK caveats only**, and **three have no full license/notice at their exact source**. All 11 Windows candidate gaps are in the full-text group. `llama-cpp-2` 0.1.151 now has its repository's exact MIT and Apache texts from commit `7f0a0d95514aebe86efab16527745852ee72931c`, including the original Dial AI copyright. Wrapper texts do not replace the separately inventoried llama.cpp/ggml vendor notices.

## Evidence and method

The collector now runs locked/offline Cargo trees for `x86_64-pc-windows-msvc` and `aarch64-apple-darwin`, using normal edges without proc-macro subtrees for runtime candidates, then including build and dev edges for comparison. It preserves each raw graph. The installed npm production tree is captured with `npm ls --omit=dev --all --json --offline`. No build or package installation is performed.

`--recover-notices` explicitly fetches only primary upstream source material for runtime candidates, normally at the commit recorded in the installed crate's `.cargo_vcs_info.json`. It considers the crate/workspace root and ancestor license/NOTICE files, not unrelated sibling packages. The recovered bytes, source URL, immutable commit, SHA256 and copied path are retained. No copyright name/year or license boilerplate was fabricated.

For old `block` 0.1.6 and `malloc_buf` 0.0.6, the crate contains no VCS receipt. Official exact-version tags were resolved, and **every packaged Rust source file** was matched to the Git blob hashes in the upstream tag tree before using those commits as evidence. Both tags still contain no ancestor license/NOTICE. Their actual Cargo.toml declarations were retained. `sigchld` 0.2.4 likewise has no ancestor license/NOTICE at its crate-recorded commit; its declaration is preserved.

| Artifact | Purpose |
|---|---|
| [dispositions.json](../../legal/third-party/reconciliation/dispositions.json) | All 130 entries, target scopes, package links, recovery status and evidence references. |
| [recoveries.json](../../legal/third-party/reconciliation/recoveries.json) | Exact upstream commit/tag provenance, copied-file hashes and declaration-only distinctions. |
| [evidence directory](../../legal/third-party/reconciliation/evidence/) | Six Cargo target graphs and the npm production tree. |
| [recovered notices](../../legal/third-party/reconciliation/recovered/) | Original license/NOTICE bytes and the three actual Cargo metadata declarations. |
| [inventory.json](../../legal/third-party/inventory.json) | Main inventory now links reconciliation and recovered material without erasing source gaps. |

## Manual decisions still required

1. Reconcile the actual installer/link outputs, native DLLs/dylibs, models and redistributables against these dependency candidates. Build-only classification is not permission to distribute a tool without its notices.
2. Decide the applicable SPDX alternative and complete all resulting obligations; retain AND obligations and exact copyright statements. Recovered files are evidence, not automatic approval.
3. Review the 10 recent objc2-family LICENSE.md files. They declare license options and describe an Apple SDK derivation caveat; they are **not full license/copyright text**. Do not treat them as interchangeable with older full MIT licenses from the same repository.
4. Review `block`, `malloc_buf` and `sigchld`: exact sources contain an MIT metadata declaration but no full notice. Determine an acceptable distribution approach with the legal/release owner; this audit does not invent an author notice or contact upstream maintainers.
5. Preserve the remaining build/other-target dispositions and revisit them if distribution expands. `third_party_notices_reviewed`, publication approval and publisher identity were not changed.

## Verification

An offline repeat produced identical inventory, dispositions and recovery files. Cached recovered notice/declaration hashes are verified on every run. The last verified hashes before this document update were:

- inventory: `e817e074189f4e89dc1d55a456a5f8e8348a4cf9d7f64023cf4860b2240419c2`
- dispositions: `69074db2a7144ecfd81ee95136788af9638c6ddfcca64ba9874099e59d6523e3`
- recoveries: `42c488d2985018ee02c557a357e6799e2670a15d651bc9447f75a39ac8de48e9`

Run `node scripts/collect-third-party-notices.mjs` to regenerate locally without network. Use `--recover-notices` for explicit immutable-source recovery, or `--refresh-models` separately for model terms snapshots.

Final verification also checked all 1,218 original copied notices, 37 recovered notice files and three actual Cargo declaration proofs against their recorded SHA256 values. All 130 disposition keys are unique and accounted for; all 11 Windows runtime candidates among the original gaps have full-license text evidence. The public-distribution and third-party-review flags remain false.
