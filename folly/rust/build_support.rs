// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(dead_code)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FOLLY_INCLUDE_DIRS: &str = "FOLLY_RUST_FOLLY_INCLUDE_DIRS";
const FOLLY_LIBRARY_DIR: &str = "FOLLY_RUST_FOLLY_LIBRARY_DIR";
const JEMALLOC_LIBRARY_DIR: &str = "FOLLY_RUST_JEMALLOC_LIBRARY_DIR";
const USE_JEMALLOC: &str = "FOLLY_RUST_USE_JEMALLOC";

pub fn probe_includes(name: &str) -> Vec<PathBuf> {
    pkg_config::Config::new()
        .cargo_metadata(true)
        .probe(name)
        .unwrap_or_else(|error| panic!("unable to resolve {name} with pkg-config: {error}"))
        .include_paths
}

pub fn folly_includes() -> Vec<PathBuf> {
    for variable in [
        FOLLY_INCLUDE_DIRS,
        FOLLY_LIBRARY_DIR,
        JEMALLOC_LIBRARY_DIR,
        USE_JEMALLOC,
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    if let Some(raw_include_dirs) = env::var_os(FOLLY_INCLUDE_DIRS) {
        let include_dirs: Vec<_> = env::split_paths(&raw_include_dirs).collect();
        if include_dirs.is_empty() {
            panic!("{FOLLY_INCLUDE_DIRS} must contain at least one directory");
        }

        emit_library("folly", FOLLY_LIBRARY_DIR);
        if env_flag(USE_JEMALLOC) {
            emit_library("jemalloc", JEMALLOC_LIBRARY_DIR);
        }
        return include_dirs;
    }

    let folly = pkg_config::Config::new()
        .cargo_metadata(true)
        .probe("libfolly")
        .unwrap_or_else(|error| {
            panic!(
                "unable to resolve libfolly with pkg-config: {error}; set \
                 PKG_CONFIG_PATH to the selected Folly installation"
            )
        });

    let allocator_is_linked = folly.libs.iter().any(|name| name == "jemalloc");
    if uses_jemalloc(&folly.include_paths) && !allocator_is_linked {
        pkg_config::Config::new()
            .cargo_metadata(true)
            .probe("jemalloc")
            .unwrap_or_else(|error| {
                panic!(
                    "the selected Folly headers enable jemalloc, but its link \
                     provider is unavailable through pkg-config: {error}"
                )
            });
    }

    folly.include_paths
}

pub fn bindgen_clang_args() -> Vec<String> {
    for variable in [
        "BINDGEN_EXTRA_CLANG_ARGS",
        "CARGO_CFG_TARGET_OS",
        "CXX",
        "MACOSX_DEPLOYMENT_TARGET",
        "SDKROOT",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    let mut args: Vec<String> = env::var("BINDGEN_EXTRA_CLANG_ARGS")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_owned)
        .collect();

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return args;
    }

    let sdk = env::var("SDKROOT")
        .ok()
        .filter(|path| !path.is_empty())
        .or_else(xcrun_sdk_path);
    if let Some(sdk) = sdk {
        if !args.iter().any(|arg| arg == "-isysroot") {
            args.extend(["-isysroot".to_owned(), sdk.clone()]);
        }
        args.extend(["-isystem".to_owned(), format!("{sdk}/usr/include/c++/v1")]);

        let compiler = env::var("CXX").unwrap_or_else(|_| "clang++".to_owned());
        if let Some(resource_dir) = command_output(&compiler, &["-print-resource-dir"]) {
            args.extend(["-isystem".to_owned(), format!("{resource_dir}/include")]);
        }

        args.extend(["-isystem".to_owned(), format!("{sdk}/usr/include")]);
    }

    if let Ok(target) = env::var("MACOSX_DEPLOYMENT_TARGET") {
        if !target.is_empty()
            && !args
                .iter()
                .any(|arg| arg.starts_with("-mmacosx-version-min="))
        {
            args.push(format!("-mmacosx-version-min={target}"));
        }
    }

    args
}

fn emit_library(name: &str, directory_variable: &str) {
    let directory = env::var(directory_variable).unwrap_or_else(|_| {
        panic!(
            "{directory_variable} is required when {FOLLY_INCLUDE_DIRS} selects \
             a CMake build-tree provider"
        )
    });
    println!("cargo:rustc-link-search=native={directory}");
    println!("cargo:rustc-link-lib=dylib={name}");
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "ON" | "TRUE" | "YES" | "on" | "true" | "yes"
            )
        })
        .unwrap_or(false)
}

fn uses_jemalloc(include_dirs: &[PathBuf]) -> bool {
    include_dirs.iter().any(|include_dir| {
        let config = include_dir.join("folly/folly-config.h");
        let Ok(contents) = fs::read_to_string(config) else {
            return false;
        };
        contents.lines().any(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            fields.as_slice() == ["#define", "FOLLY_USE_JEMALLOC", "1"]
        })
    })
}

fn xcrun_sdk_path() -> Option<String> {
    command_output("xcrun", &["--show-sdk-path"])
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(Path::new(program)).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}
