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
    tauri_build::build()
}
