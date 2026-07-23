use std::env;
use std::path::PathBuf;

fn probe_includes(name: &str) -> Vec<PathBuf> {
    pkg_config::probe_library(name)
        .map(|lib| lib.include_paths)
        .unwrap_or_default()
}

fn main() {
    let fmt_includes = probe_includes("fmt");
    let folly_includes = probe_includes("libfolly");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut build = cxx_build::bridge("lib.rs");

    let cxx_dir = out_dir.join("cxxbridge/include");
    let target_dir = cxx_dir.join("folly/rust/socket_address");
    std::fs::create_dir_all(&target_dir).ok();
    std::fs::copy(
        cxx_dir.join("socket_address/lib.rs.h"),
        target_dir.join("lib.rs.h"),
    ).ok();

    build
        .file("RustSocketAddress.cpp")
        .include("../../..")
        .include(&cxx_dir);

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
        .compile("socket_address");
}
