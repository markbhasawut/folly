use bindgen::callbacks::{MacroParsingBehavior, ParseCallbacks};
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

#[path = "../build_support.rs"]
mod build_support;

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

fn main() {
    let folly_includes = build_support::folly_includes();
    let fmt_includes = build_support::probe_includes("fmt");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut build = cxx_build::bridge("tdigest.rs");
    build.file("tdigest.cpp").include("../../..");

    for path in folly_includes.iter().chain(fmt_includes.iter()) {
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
            bindgen::CodegenConfig::TYPES
                | bindgen::CodegenConfig::FUNCTIONS
                | bindgen::CodegenConfig::VARS,
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

    for arg in build_support::bindgen_clang_args() {
        builder = builder.clang_arg(arg);
    }

    for path in folly_includes.iter().chain(fmt_includes.iter()) {
        if !path.to_string_lossy().contains("/usr/include") {
            builder = builder.clang_arg(format!("-I{}", path.display()));
        }
    }

    let bindings = builder
        .generate()
        .expect("Unable to generate bindings for tdigest");

    let out_file = out_dir.join("bindings.rs");
    bindings
        .write_to_file(&out_file)
        .expect("Couldn't write bindings!");
}
