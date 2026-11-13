# Notice-source reconciliation

This is a classification of the original 130 source-package gaps, **not proof that all these packages ship and not legal approval**. Default features and the locked Windows x64 / macOS arm64 graphs were used. Runtime candidate graphs exclude build edges and proc-macro subtrees; raw graphs and the npm production tree are preserved in evidence/.

| Disposition | Count |
|---|---:|
| npm production-tree gaps | 0 |
| npm installed build tools | 4 |
| npm optional build packages not installed | 59 |
| Cargo runtime candidates, either target | 28 |
| Cargo build/proc-macro only | 4 |
| Cargo dev only | 0 |
| Cargo other target or inactive | 35 |

Within runtime candidates: 15 have exact upstream full-license text recovered; 10 have upstream licensing declarations/Apple SDK caveats only; 3 have no full license/notice recovered. All 11 Windows runtime source gaps have recovered material; inspect each row for scope. Unused platform packages were not downloaded.

## Runtime candidate detail

| Crate | Windows x64 | macOS arm64 | Material recovered / pending |
|---|---|---|---|
| alloc-stdlib 0.2.2 | Yes | Yes | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| block 0.1.6 | No | Yes | metadata_declaration_only_no_full_license_or_copyright_notice_recovered |
| block-sys 0.2.1 | No | Yes | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| block2 0.4.0 | No | Yes | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| block2 0.6.2 | No | Yes | licensing_declaration_only_manual_terms_and_sdk_review_required |
| clipboard-win 5.4.1 | Yes | No | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| dasp_sample 0.11.0 | Yes | Yes | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| dispatch2 0.3.1 | No | Yes | licensing_declaration_only_manual_terms_and_sdk_review_required |
| llama-cpp-2 0.1.151 | Yes | Yes | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| malloc_buf 0.0.6 | No | Yes | metadata_declaration_only_no_full_license_or_copyright_notice_recovered |
| objc-sys 0.3.5 | No | Yes | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| objc2 0.5.2 | No | Yes | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| objc2 0.6.4 | No | Yes | licensing_declaration_only_manual_terms_and_sdk_review_required |
| objc2-app-kit 0.3.2 | No | Yes | licensing_declaration_only_manual_terms_and_sdk_review_required |
| objc2-core-foundation 0.3.2 | No | Yes | licensing_declaration_only_manual_terms_and_sdk_review_required |
| objc2-core-graphics 0.3.2 | No | Yes | licensing_declaration_only_manual_terms_and_sdk_review_required |
| objc2-encode 4.1.0 | No | Yes | licensing_declaration_only_manual_terms_and_sdk_review_required |
| objc2-exception-helper 0.1.1 | No | Yes | licensing_declaration_only_manual_terms_and_sdk_review_required |
| objc2-osa-kit 0.3.2 | No | Yes | licensing_declaration_only_manual_terms_and_sdk_review_required |
| objc2-web-kit 0.3.2 | No | Yes | licensing_declaration_only_manual_terms_and_sdk_review_required |
| sigchld 0.2.4 | No | Yes | metadata_declaration_only_no_full_license_or_copyright_notice_recovered |
| unic-char-property 0.9.0 | Yes | Yes | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| unic-char-range 0.9.0 | Yes | Yes | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| unic-common 0.9.0 | Yes | Yes | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| unic-ucd-ident 0.9.0 | Yes | Yes | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| unic-ucd-version 0.9.0 | Yes | Yes | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| webview2-com 0.38.2 | Yes | No | full_license_text_recovered_legal_selection_and_artifact_review_still_required |
| webview2-com-sys 0.38.2 | Yes | No | full_license_text_recovered_legal_selection_and_artifact_review_still_required |

## Remaining release review

1. Reconcile the actual linked/bundled artifacts with this graph, including static native code, DLLs/dylibs, SDK/runtime redistributables and model weights. A graph can over-include feature or linker-eliminated code.
2. Decide and record applicable license alternatives and obligations. Preserve AND expressions and exact copyright text; recovered files do not themselves prove compliance.
3. Review the objc2 LICENSE.md declarations and Apple SDK caveat. These files link standard license terms but are not complete license/copyright notices; do not silently substitute a guessed MIT header.
4. Review block 0.1.6, malloc_buf 0.0.6 and sigchld 0.2.4. Their exact release/commit sources declare MIT but contain no ancestor LICENSE/NOTICE; corresponding Cargo.toml declarations are retained. The first two exact tags were bound to installed crates by matching every packaged Rust source blob. No copyright year or owner was invented.
5. Build-only/other-target gaps are not dropped: retain their dispositions and review if those tools, source packages or platforms are later distributed. No publisher identity, publication approval or third_party_notices_reviewed flag was set.

Use node scripts/collect-third-party-notices.mjs for reproducible offline regeneration and cached-byte verification. --recover-notices explicitly fetches primary upstream source at exact crate commits/tags; --refresh-models separately refreshes model terms. Raw source gaps remain in ../missing-notices.json so recovered evidence does not erase the original finding.
