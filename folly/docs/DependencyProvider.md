# Single-provider dependency builds

Folly exports `MetaDependencyProvider.cmake` for projects that compose the
Meta C++ stack:

- jemalloc
- Folly
- Fizz
- Wangle
- mvfst
- Proxygen
- FBThrift
- fb303

These libraries must come from one coherent install prefix. Loading two
jemalloc dylibs is undefined at the process boundary: memory allocated through
one implementation can be released through the other. Mixing revisions of the
other libraries also violates their intentionally unstable C++ ABI.

Unrelated dependencies may come from another prefix. For example, a stack
installed under `/usr/local` may use OpenSSL, Boost, c-ares, fmt, gflags, glog,
or GoogleTest from Homebrew.

## CMake contract

On macOS, `META_ENFORCE_SINGLE_DEPENDENCY_PREFIX` defaults to `ON`. Each
participating package validates:

1. the locations of its imported CMake targets; and
2. absolute Meta-stack dependencies recorded in each imported Mach-O image.

The second check detects a stale library such as `/usr/local/lib/libmvfst`
that still records `/opt/homebrew/.../libjemalloc` in its load commands.

`META_DEPENDENCY_PREFIX` overrides the prefix inferred from the package's
anchor target:

```sh
cmake -S . -B build \
  -DCMAKE_PREFIX_PATH="/usr/local;/opt/homebrew" \
  -DMETA_DEPENDENCY_PREFIX=/usr/local
```

Do not use the override to mask a mixed closure. Rebuild the first offending
package and configure consumers from a clean build directory. Disable the
check only for deliberate source superbuilds whose in-tree targets have no
installed location:

```sh
cmake -S . -B build \
  -DMETA_ENFORCE_SINGLE_DEPENDENCY_PREFIX=OFF
```

Folly's installed targets keep their headers in CMake's normal include search
tier rather than the imported-system tier. This is intentional: a dependency
such as glog may export the Homebrew umbrella directory
`/opt/homebrew/include`, which can contain a second Folly installation. If
that umbrella precedes `/usr/local/include`, compilation may use Homebrew's
`folly-config.h` while linking `/usr/local/lib/libfolly`, producing an ABI
mismatch even though the Mach-O closure is clean. Do not manually reclassify
the selected Folly include directory as `SYSTEM`, and avoid adding umbrella
include prefixes globally.

## Shared and static modes

`BUILD_SHARED_LIBS=ON` is the easiest mode to audit on macOS because `otool -L`
shows the resolved dylib closure. With `BUILD_SHARED_LIBS=OFF`, the provider
check still validates imported archive locations, but the final executable
owns dependency resolution.

A static Meta stack that links dynamic OpenSSL or other system packages is a
mixed-linkage build, not a fully static executable. A fully static build
requires static variants of every transitive dependency and platform support
for static system runtime linkage. Generated FBThrift registration objects may
also require whole-archive linkage so their startup initializers are retained.

Audit a final macOS executable with:

```sh
otool -L ./your_binary
```

There must be no `/opt/homebrew` entry whose basename begins with
`libjemalloc`, `libfolly`, `libfizz`, `libwangle`, `libmvfst`, `libproxygen`,
`libthrift`, `libfbthrift`, or `libfb303` when `/usr/local` is the selected
Meta dependency prefix.

## LLVM Clang on macOS

Folly exposes two coherent C++ runtime choices when it is built with upstream
LLVM Clang on macOS. Both still use an Apple SDK for C and platform headers:

- `FOLLY_CXX_RUNTIME_PROVIDER=APPLE` uses libc++, libc++abi, and libunwind from
  the selected Xcode SDK and macOS runtime.
- `FOLLY_CXX_RUNTIME_PROVIDER=LLVM` uses the matched libc++, libc++abi, and
  libunwind shipped under `FOLLY_LLVM_ROOT`.

Do not combine headers from one provider with libraries from the other. In
particular, adding only Homebrew LLVM's `-lunwind` to an Apple-runtime build is
not supported.

Install the compiler and the separately packaged Mach-O linker first:

```sh
brew install llvm lld
```

Homebrew LLVM does not replace the macOS SDK. Install either full Xcode or the
Command Line Tools, then verify all three inputs before configuring:

```sh
xcode-select -p
xcrun --sdk macosx --show-sdk-path
/opt/homebrew/opt/llvm/bin/clang++ --version
/opt/homebrew/opt/lld/bin/ld64.lld --version
```

When multiple Xcode versions are installed, either select one with
`xcode-select` or set `DEVELOPER_DIR` for the configure invocation. Pass the
absolute result of `xcrun --sdk macosx --show-sdk-path` as
`CMAKE_OSX_SYSROOT`.

`CMAKE_OSX_DEPLOYMENT_TARGET` is the oldest macOS version on which the output
must run; it is not the SDK version. The examples use `27.0` because they
target SDK 27 and intentionally require macOS 27. A build intended for an
older supported macOS release should set that release explicitly, for
example `-DCMAKE_OSX_DEPLOYMENT_TARGET=26.0`, while continuing to use an SDK
whose deployment range includes it. Keep the same deployment target across
jemalloc, Folly, and every downstream Meta C++ package. Python extensions also
inherit this value so their wheel and Mach-O deployment metadata agree with
the C++ libraries.

The Apple-runtime configuration is:

```sh
cmake -S . -B build-apple-runtime -G Ninja \
  -DCMAKE_C_COMPILER=/opt/homebrew/opt/llvm/bin/clang \
  -DCMAKE_CXX_COMPILER=/opt/homebrew/opt/llvm/bin/clang++ \
  -DCMAKE_OSX_SYSROOT=/path/to/MacOSX.sdk \
  -DCMAKE_OSX_DEPLOYMENT_TARGET=27.0 \
  -DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=/opt/homebrew/opt/lld/bin/ld64.lld \
  -DCMAKE_SHARED_LINKER_FLAGS=-fuse-ld=/opt/homebrew/opt/lld/bin/ld64.lld \
  -DCMAKE_MODULE_LINKER_FLAGS=-fuse-ld=/opt/homebrew/opt/lld/bin/ld64.lld \
  -DFOLLY_CXX_RUNTIME_PROVIDER=APPLE
```

The full LLVM-runtime configuration is:

```sh
cmake -S . -B build-llvm-runtime -G Ninja \
  -DCMAKE_C_COMPILER=/opt/homebrew/opt/llvm/bin/clang \
  -DCMAKE_CXX_COMPILER=/opt/homebrew/opt/llvm/bin/clang++ \
  -DCMAKE_OSX_SYSROOT=/path/to/MacOSX.sdk \
  -DCMAKE_OSX_DEPLOYMENT_TARGET=27.0 \
  -DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=/opt/homebrew/opt/lld/bin/ld64.lld \
  -DCMAKE_SHARED_LINKER_FLAGS=-fuse-ld=/opt/homebrew/opt/lld/bin/ld64.lld \
  -DCMAKE_MODULE_LINKER_FLAGS=-fuse-ld=/opt/homebrew/opt/lld/bin/ld64.lld \
  -DFOLLY_CXX_RUNTIME_PROVIDER=LLVM \
  -DFOLLY_LLVM_ROOT=/opt/homebrew/opt/llvm
```

The LLVM mode adds both LLVM runtime library directories to the link search
path and runtime search path. Audit every installed dylib and executable:

```sh
otool -L /path/to/libfolly.dylib
```

Full LLVM mode also requires every C++ dependency in the process to use that
same runtime. Homebrew bottles normally load Apple's `/usr/lib/libc++.1.dylib`;
they cannot be combined with an LLVM-runtime Folly. Rebuild at least Boost,
fmt, glog, gflags, snappy, ICU, and GoogleTest with the same LLVM headers,
libraries, deployment target, and runtime search paths. Rebuild any nominally
C-only library too if its installed dylib nevertheless records Apple libc++;
the current dependency audit is authoritative. Folly validates its imported
dylibs during configuration and rejects this mixed-runtime closure before
compilation. C-only dependencies such as OpenSSL and libevent normally do not
need a C++ runtime rebuild.

Apple mode must resolve the platform C++ runtime from `/usr/lib`. LLVM mode
must resolve `libc++`, `libc++abi`, and `libunwind` from the selected
`FOLLY_LLVM_ROOT`; the installed CMake and pkg-config interfaces export the
matching LLVM include, link-search, and runtime-search paths. Applications
must retain those runtime search paths. Treat the two installed variants as
different ABI packages rather than switching runtime providers downstream.

`FOLLY_USE_SYMBOLIZER` is independent of the CMake build type and of the
runtime provider. It controls Folly's in-process ELF/DWARF symbolizer, so it is
disabled on Apple platforms. LLDB still uses Mach-O DWARF or dSYM information;
keep compiler debug information enabled when debugging either runtime mode.
