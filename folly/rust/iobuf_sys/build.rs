use bindgen::callbacks::{MacroParsingBehavior, ParseCallbacks};
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

#[path = "../build_support.rs"]
mod build_support;

// Workaround for https://github.com/rust-lang/rust-bindgen/issues/687
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
    let gflags_includes = build_support::probe_includes("gflags");
    let fmt_includes = build_support::probe_includes("fmt");
    let glog_includes = build_support::probe_includes("libglog");

    let mut builder = bindgen::Builder::default()
        .header("../iobuf/iobuf.h")
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
        .allowlist_function(".*iobuf.*")
        .allowlist_type("folly::IOBuf.*")
        .allowlist_type("facebook::rust::.*")
        .allowlist_var("facebook::rust::.*")
        .blocklist_type("folly::fbvector.*")
        .blocklist_type("__type")
        .blocklist_type("type_")
        .opaque_type("(::)?std::.*")
        .opaque_type("folly::fbstring.*")
        .clang_arg("-I../../..");

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

    for path in folly_includes
        .iter()
        .chain(gflags_includes.iter())
        .chain(fmt_includes.iter())
        .chain(glog_includes.iter())
    {
        if !path.to_string_lossy().contains("/usr/include") {
            builder = builder.clang_arg(format!("-I{}", path.display()));
        }
    }

    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_file = out_path.join("bindings.rs");
    std::fs::create_dir_all(&out_path).expect("Couldn't create output directory for bindings");
    bindings
        .write_to_file(&out_file)
        .expect("Couldn't write bindings!");
}
