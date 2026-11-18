use std::path::{Path, PathBuf};

fn main() {
    // macOS: llama-cpp-2's mandatory dynamic-link build emits dylibs the exe
    // references via @rpath (libllama, libggml, libggml-base, libllama-common).
    // `cargo run`/`tauri dev` inject loader paths so it works there, but a
    // standalone binary has no LC_RPATH and dyld aborts at launch. Search next
    // to the exe (dev: target/<profile>/ holds the dylibs) and in the .app
    // bundle's Frameworks dir (release: ship the dylibs there).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    }
    stage_msvc_runtime();
    tauri_build::build()
}

/// MSVC runtime DLLs the shipped binaries import at load time. chirp.exe, llama/ggml and the
/// ggml backends are built with the dynamic CRT, and whisper/ggml use OpenMP (vcomp140). A
/// clean Windows install has none of these, and the VC++ Redistributable needs admin rights,
/// which the per-user one-click installer must not ask for. Microsoft supports app-local
/// deployment of these exact files from the VC\Redist folder. `scripts/check-windows-bundle.mjs`
/// fails the bundle if a shipped binary ever imports a runtime DLL that is not in this list.
const CRT_DLLS: &[&str] = &["msvcp140.dll", "vcruntime140.dll", "vcruntime140_1.dll"];
const OPENMP_DLLS: &[&str] = &["vcomp140.dll"];

/// Copies the MSVC runtime next to the exe in `target/<profile>/`, where
/// `tauri.windows.conf.json` picks the files up as bundle resources.
fn stage_msvc_runtime() {
    let var = |name: &str| std::env::var(name).unwrap_or_default();
    if var("CARGO_CFG_TARGET_OS") != "windows" || var("CARGO_CFG_TARGET_ENV") != "msvc" {
        return;
    }
    let arch = match var("CARGO_CFG_TARGET_ARCH").as_str() {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "x86",
        other => panic!("no MSVC redistributable mapping for target arch {other}"),
    };
    println!("cargo:rerun-if-env-changed=VCToolsRedistDir");
    let redist = redist_dir().unwrap_or_else(|| {
        panic!(
            "MSVC redistributable folder (VC\\Redist\\MSVC\\<version>) not found. Build from a \
             vcvars64 environment (test-release.cmd / build-release.cmd do this) or install the \
             MSVC C++ build tools. Lectus ships these DLLs next to chirp.exe."
        )
    });
    let arch_dir = redist.join(arch);
    let crt = component_dir(&arch_dir, ".CRT");
    let openmp = component_dir(&arch_dir, ".OpenMP");
    let out_dir = PathBuf::from(var("OUT_DIR"));
    // OUT_DIR = <target>/<profile>/build/<pkg>-<hash>/out
    let profile_dir = out_dir.ancestors().nth(3).expect("OUT_DIR has a profile ancestor");
    for (dir, names) in [(&crt, CRT_DLLS), (&openmp, OPENMP_DLLS)] {
        for name in names.iter().filter(|n| arch != "x86" || **n != "vcruntime140_1.dll") {
            let src = dir.join(name);
            let dst = profile_dir.join(name);
            println!("cargo:rerun-if-changed={}", src.display());
            println!("cargo:rerun-if-changed={}", dst.display());
            if same_file_contents(&src, &dst) {
                continue;
            }
            // fs::copy keeps the source's write time on Windows, so the
            // rerun-if-changed on `dst` does not retrigger every build.
            std::fs::copy(&src, &dst).unwrap_or_else(|e| {
                panic!("copy {} -> {}: {e}", src.display(), dst.display())
            });
        }
    }
}

/// `VCToolsRedistDir` from a vcvars environment, else the latest Visual Studio install that has
/// the C++ tools, located with vswhere and its default redist version file.
fn redist_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("VCToolsRedistDir").map(PathBuf::from) {
        if dir.is_dir() {
            return Some(dir);
        }
    }
    let program_files = std::env::var_os("ProgramFiles(x86)")?;
    let vswhere = Path::new(&program_files).join("Microsoft Visual Studio/Installer/vswhere.exe");
    let output = std::process::Command::new(vswhere)
        .args(["-latest", "-products", "*", "-requires"])
        .arg("Microsoft.VisualStudio.Component.VC.Tools.x86.x64")
        .args(["-property", "installationPath"])
        .output()
        .ok()?;
    let install = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let version_file = install.join("VC/Auxiliary/Build/Microsoft.VCRedistVersion.default.txt");
    let version = std::fs::read_to_string(version_file).ok()?;
    let dir = install.join("VC/Redist/MSVC").join(version.trim());
    dir.is_dir().then_some(dir)
}

/// `Microsoft.VC143.CRT` today; the toolset number changes with Visual Studio releases.
fn component_dir(arch_dir: &Path, suffix: &str) -> PathBuf {
    let mut found: Vec<PathBuf> = std::fs::read_dir(arch_dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", arch_dir.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n.starts_with("Microsoft.VC") && n.ends_with(suffix) && path.is_dir()
            })
        })
        .collect();
    found.sort();
    found.pop().unwrap_or_else(|| panic!("no Microsoft.VC*{suffix} folder in {}", arch_dir.display()))
}

fn same_file_contents(a: &Path, b: &Path) -> bool {
    match (std::fs::read(a), std::fs::read(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}
