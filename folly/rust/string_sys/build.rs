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

    let mut builder = bindgen::Builder::default()
        .header("../string/string.h")
        .parse_callbacks(Box::new(IgnoreMacros::new()))
        .with_codegen_config(
            bindgen::CodegenConfig::TYPES |
            bindgen::CodegenConfig::FUNCTIONS |
            bindgen::CodegenConfig::VARS
        )
        .clang_arg("-x")
        .clang_arg("c++")
        .clang_arg("-DGLOG_USE_GLOG_EXPORT")
        .generate_comments(false)

        .allowlist_function("facebook::rust::.*")
        .allowlist_type("facebook::rust::.*")
        .allowlist_type("folly::StringPiece")
        .allowlist_type("folly::MutableStringPiece")
        .allowlist_type("folly::ByteRange")
        .allowlist_type("folly::MutableByteRange")
        .allowlist_var("facebook::rust::.*")

        .opaque_type("folly::StringPiece")
        .opaque_type("folly::MutableStringPiece")
        .opaque_type("folly::ByteRange")
        .opaque_type("folly::MutableByteRange")

        .clang_arg("-I../../..");

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
        .expect("Unable to generate bindings for string");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_file = out_path.join("bindings.rs");
    std::fs::create_dir_all(&out_path).expect("Couldn't create output dir");
    bindings.write_to_file(&out_file).expect("Couldn't write bindings!");

    let mut cc = cc::Build::new();
    cc.cpp(true)
        .file("../string/string.cpp")
        .include("../../..");
    for path in fmt_includes.iter().chain(folly_includes.iter()) {
        if !path.to_string_lossy().contains("/usr/include") {
            cc.include(path);
        }
    }
    if cfg!(target_os = "windows") {
        cc.flag_if_supported("/std:c++20");
    } else {
        cc.flag_if_supported("-std=c++20");
    }
    cc.define("GLOG_USE_GLOG_EXPORT", None);
    cc.compile("string_ffi");
}
