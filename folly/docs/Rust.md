# Folly Rust bindings

Folly's Rust code is a Cargo workspace under `folly/rust`. The crates use
`cxx`, `cc`, and bindgen build scripts to compile bridges against the selected
Folly C++ installation. They are source packages, not a stable Rust or C++ ABI:
rebuild the workspace whenever Folly, the C++ runtime, compiler, SDK, or
allocator provider changes.

The supported OSS toolchain is pinned by `folly/rust/rust-toolchain.toml` to
`nightly-2026-05-22`. Install it with rustup before configuring CMake:

```sh
rustup toolchain install nightly-2026-05-22 \
  --profile minimal --component clippy --component rustfmt
```

The workspace keeps release-profile stripping disabled. This pinned nightly
otherwise strips loadable crate metadata from macOS proc-macro dylibs and
downstream compilation fails with `E0463`. Installed deliverables are source,
not those validation artifacts; strip final downstream products after their
Rust link if required.

## Direct Cargo build

Select one Folly installation through pkg-config. Put its prefix before any
package-manager defaults; otherwise Cargo can compile against one
`folly-config.h` and load another `libfolly`, which is an ABI violation.

```sh
export FOLLY_PREFIX=/path/to/prefix
export PKG_CONFIG_PATH="$FOLLY_PREFIX/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

cd folly/rust
rustup run nightly-2026-05-22 cargo test --workspace --locked
```

The build scripts inspect the selected `folly-config.h`. If Folly enables
jemalloc, the same jemalloc provider is added to the Rust link closure. A
missing allocator is a configuration error rather than a deferred `mallctl`
link failure. The selected Folly include directory is applied before umbrella
dependency paths such as Homebrew's `/opt/homebrew/include`.

On macOS, use the same compiler, SDK, and deployment target as the C++ build:

```sh
export CC=/path/to/llvm/bin/clang
export CXX=/path/to/llvm/bin/clang++
export SDKROOT=/absolute/path/to/MacOSX.sdk
export MACOSX_DEPLOYMENT_TARGET=27.0

rustup run nightly-2026-05-22 cargo test --workspace --locked
```

`MACOSX_DEPLOYMENT_TARGET` is the oldest supported runtime, not the SDK
version. Choose a lower value when the selected SDK supports it. Bindgen uses
`SDKROOT`, `CXX`, and the compiler resource directory; extra target-specific
arguments can be supplied through `BINDGEN_EXTRA_CLANG_ARGS`.

## CMake integration

Rust is opt-in and currently requires shared Folly. CMake owns the dependency
provider and injects its build-tree Folly, jemalloc, compiler, runtime, SDK,
deployment target, linker, and rpaths into Cargo:

```sh
cmake -S . -B build -G Ninja \
  -DBUILD_SHARED_LIBS=ON \
  -DBUILD_TESTS=ON \
  -DFOLLY_BUILD_RUST_BINDINGS=ON \
  -DFOLLY_RUST_TOOLCHAIN=nightly-2026-05-22 \
  -DFOLLY_RUST_CARGO_PROFILE=release

cmake --build build --target folly_rust_bindings
ctest --test-dir build -R '^folly_rust_workspace$' --output-on-failure
```

The target is part of the default build when enabled. `debug` and `release`
are the accepted values of `FOLLY_RUST_CARGO_PROFILE`. CMake deliberately does
not export Cargo `.rlib` files as imported CMake targets: they are tied to the
exact rustc and dependency graph and are not a distribution ABI.

## Installed workspace

An enabled build installs the Cargo workspace at
`${CMAKE_INSTALL_DATADIR}/folly/rust`, normally
`share/folly/rust`. `find_package(folly CONFIG REQUIRED)` exports:

- `FOLLY_HAVE_RUST_BINDINGS`, indicating whether the workspace was installed;
- `FOLLY_RUST_WORKSPACE_DIR`, its absolute installed path; and
- `FOLLY_RUST_TOOLCHAIN`, the validated rustup toolchain.

Downstream Cargo packages can use individual crates by path from that
workspace. Keep `PKG_CONFIG_PATH` pointed at the same installation prefix so
their bridge builds resolve the matching C++ headers and dylib. Install source
is intentional: downstreams rebuild the bindings for their rustc and target
instead of consuming compiler-private artifacts.

For provider and LLVM-versus-Apple C++ runtime rules, see
`folly/docs/DependencyProvider.md`. The entire final process must use one C++
runtime and one Meta dependency prefix.
