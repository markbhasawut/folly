use std::env;
use std::path::PathBuf;

#[path = "../build_support.rs"]
mod build_support;

fn main() {
    let folly_includes = build_support::folly_includes();
    let fmt_includes = build_support::probe_includes("fmt");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut build = cxx_build::bridge("lib.rs");

    let cxx_dir = out_dir.join("cxxbridge/include");
    let target_dir = cxx_dir.join("folly/rust/socket_address");
    std::fs::create_dir_all(&target_dir).ok();
    std::fs::copy(
        cxx_dir.join("socket_address/lib.rs.h"),
        target_dir.join("lib.rs.h"),
    )
    .ok();

    build
        .file("RustSocketAddress.cpp")
        .include("../../..")
        .include(&cxx_dir);

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
        .compile("socket_address");
}
