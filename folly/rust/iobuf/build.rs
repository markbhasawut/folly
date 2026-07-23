use std::path::PathBuf;

fn probe_includes(name: &str) -> Vec<PathBuf> {
    pkg_config::probe_library(name)
        .map(|lib| lib.include_paths)
        .unwrap_or_default()
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(fbcode_build)");

    let fmt_includes = probe_includes("fmt");
    let folly_includes = probe_includes("libfolly");

    let mut build = cxx_build::bridge("src/lib.rs");
    build
        .file("iobuf.cpp")
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
        .compile("iobuf");
}
