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
