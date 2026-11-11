fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
        return;
    }

    // ThorVG does not link a C++ runtime on Android. Bundle the NDK's static
    // runtime so the APK does not need a separate libc++_shared.so.
    let compiler = cc::Build::new().cpp(true).get_compiler();
    let output = std::process::Command::new(compiler.path())
        .arg("--print-file-name=libc++_static.a")
        .output()
        .expect("locate the NDK C++ runtime");
    assert!(output.status.success(), "locate the NDK C++ runtime");
    let path = std::path::PathBuf::from(
        String::from_utf8(output.stdout)
            .expect("NDK runtime path is UTF-8")
            .trim(),
    );
    assert!(
        path.is_file(),
        "NDK C++ runtime is missing: {}",
        path.display()
    );
    // Do not add the whole NDK library directory: it also contains libc.a,
    // which would shadow Android's API-specific shared libc.so at link time.
    let out =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo output directory"));
    for library in ["c++_static", "c++abi"] {
        let name = format!("lib{library}.a");
        let source = path.parent().expect("NDK runtime directory").join(&name);
        std::fs::copy(&source, out.join(name)).expect("stage the NDK C++ runtime");
        println!("cargo:rerun-if-changed={}", source.display());
        println!("cargo:rustc-link-lib=static={library}");
    }
    println!("cargo:rustc-link-search=native={}", out.display());
}
