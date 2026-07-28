#[path = "../build_support.rs"]
mod build_support;

fn main() {
    let folly_includes = build_support::folly_includes();
    let fmt_includes = build_support::probe_includes("fmt");

    let mut build = cxx_build::bridge("compression.rs");
    build.file("compression.cpp").include("../../..");

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
        .compile("folly_compression");
}
