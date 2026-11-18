# Windows runtime files

Written by hand; `scripts/collect-third-party-notices.mjs` does not generate this file.

## Shipped next to chirp.exe

The Windows installer ships these Microsoft Visual C++ runtime files, because chirp.exe, the llama/ggml DLLs and the ggml backends import them at load time and a clean Windows does not have them. `src-tauri/build.rs` copies them from the Visual Studio `VC\Redist\MSVC\<version>` folder of the build machine, and `scripts/check-windows-bundle.mjs` fails the bundle if a shipped binary imports a runtime DLL the installer does not carry.

| File | Source folder | Licence |
|---|---|---|
| msvcp140.dll | Microsoft.VC143.CRT | Microsoft Visual Studio license terms, Distributable Code |
| vcruntime140.dll | Microsoft.VC143.CRT | same |
| vcruntime140_1.dll | Microsoft.VC143.CRT | same |
| vcomp140.dll | Microsoft.VC143.OpenMP | same |

The installer keeps these copies only when Windows lacks the runtime or has an older one. When System32 already has all four files at the same or a newer version, it removes the app-local copies, so the app and every DLL loaded into it use Microsoft's serviced system runtime.

Microsoft and its suppliers keep all rights in these files. The Lectus Terms pass on the end-user limits the Distributable Code terms ask for (use only with Lectus; no modification or reverse engineering where the law allows that limit).

Review before publication: confirm the redistribution right for the exact Visual Studio edition that builds the release (Build Tools on the local machine, the GitHub runner's Visual Studio in CI).

## Used from Windows, not shipped

- `vulkan-1.dll`, the Khronos Vulkan loader (Apache-2.0), comes with the graphics driver. chirp.exe needs it to start, so the installer stops with a message when it is missing. Khronos discourages bundling the loader with an application.
- The Microsoft Edge WebView2 Runtime comes with Windows 11, or the installer downloads Microsoft's bootstrapper.
