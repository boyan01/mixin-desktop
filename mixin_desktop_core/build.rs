use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let pubspec = manifest_dir.join("../flutter/pubspec.yaml");
    println!("cargo:rerun-if-changed={}", pubspec.display());

    let content = fs::read_to_string(&pubspec)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", pubspec.display()));
    let version = content
        .lines()
        .find_map(|line| line.trim().strip_prefix("version:"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| panic!("missing version in {}", pubspec.display()));
    let (build_name, build_number) = version
        .split_once('+')
        .unwrap_or_else(|| panic!("version must include a build number: {version}"));

    println!("cargo:rustc-env=MIXIN_APP_VERSION={build_name}");
    println!("cargo:rustc-env=MIXIN_APP_BUILD_NUMBER={build_number}");
}
