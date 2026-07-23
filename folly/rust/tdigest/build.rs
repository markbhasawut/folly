use bindgen::callbacks::{MacroParsingBehavior, ParseCallbacks};
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

const IGNORE_MACROS: [&str; 10] = [
    "FP_INFINITE",
    "FP_INT_DOWNWARD",
    "FP_INT_TONEAREST",
    "FP_INT_TONEARESTFROMZERO",
    "FP_INT_TOWARDZERO",
    "FP_INT_UPWARD",
    "FP_NAN",
    "FP_NORMAL",
    "FP_SUBNORMAL",
    "FP_ZERO",
];

#[derive(Debug)]
struct IgnoreMacros(HashSet<String>);

impl ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        if self.0.contains(name) {
            MacroParsingBehavior::Ignore
        } else {
            MacroParsingBehavior::Default
        }
    }
}

impl IgnoreMacros {
    fn new() -> Self {
        Self(IGNORE_MACROS.into_iter().map(|s| s.to_owned()).collect())
    }
}

fn probe_includes(name: &str) -> Vec<PathBuf> {
    pkg_config::probe_library(name)
        .map(|lib| lib.include_paths)
        .unwrap_or_default()
}

fn main() {
    let fmt_includes = probe_includes("fmt");
    let folly_includes = probe_includes("libfolly");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut build = cxx_build::bridge("tdigest.rs");
    build
        .file("tdigest.cpp")
        .include("../../..");

    for path in fmt_includes.iter().chain(folly_includes.iter()) {
        if !path.to_string_lossy().contains("/usr/include") {
            build.include(path);
        }
    }

    if cfg!(target_os = "windows") {
        build.flag_if_supported("/std:c++20");
        build.flag_if_supported("/EHsc");
    } else {
        build.flag_if_supported("-std=c++20");
    }

    build
        .define("GLOG_USE_GLOG_EXPORT", None)
        .compile("tdigest");

    let cxx_header_dir = out_dir.join("cxxbridge/include");

    let mut builder = bindgen::Builder::default()
        .header("tdigest.h")
        .parse_callbacks(Box::new(IgnoreMacros::new()))
        .with_codegen_config(
            bindgen::CodegenConfig::TYPES |
            bindgen::CodegenConfig::FUNCTIONS |
            bindgen::CodegenConfig::VARS
        )
        .clang_arg("-x")
        .clang_arg("c++")
        .clang_arg("-DGLOG_USE_GLOG_EXPORT")
        .enable_cxx_namespaces()
        .generate_comments(false)

        .allowlist_function("facebook::rust::.*")
        .allowlist_type("folly::TDigest")
        .allowlist_type("facebook::rust::.*")
        .allowlist_var("facebook::rust::.*")

        .opaque_type("std::.*")

        .clang_arg("-I../../..")
        .clang_arg(format!("-I{}", cxx_header_dir.display()));

    if cfg!(target_os = "windows") {
        builder = builder
            .clang_arg("-std=c++20")
            .clang_arg("-fms-compatibility")
            .clang_arg("-fms-extensions");
    } else {
        builder = builder.clang_arg("-std=c++20");
    }

    if cfg!(target_os = "macos") {
        if let Ok(output) = std::process::Command::new("xcrun").arg("--show-sdk-path").output() {
            let sdk = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !sdk.is_empty() {
                builder = builder
                    .clang_arg("-isystem")
                    .clang_arg(format!("{}/usr/include/c++/v1", sdk));
            }
        }
        if let Ok(output) = std::process::Command::new("clang").arg("-print-resource-dir").output() {
            let res_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !res_dir.is_empty() {
                builder = builder
                    .clang_arg("-isystem")
                    .clang_arg(format!("{}/include", res_dir));
            }
        }
        if let Ok(output) = std::process::Command::new("xcrun").arg("--show-sdk-path").output() {
            let sdk = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !sdk.is_empty() {
                builder = builder
                    .clang_arg("-isystem")
                    .clang_arg(format!("{}/usr/include", sdk));
            }
        }
    }

    for path in fmt_includes.iter().chain(folly_includes.iter()) {
        if !path.to_string_lossy().contains("/usr/include") {
            builder = builder.clang_arg(format!("-I{}", path.display()));
        }
    }

    let bindings = builder
        .generate()
        .expect("Unable to generate bindings for tdigest");

    let out_file = out_dir.join("bindings.rs");
    bindings.write_to_file(&out_file).expect("Couldn't write bindings!");
}
