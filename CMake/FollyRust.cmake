# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

function(folly_enable_rust_bindings)
  if(NOT BUILD_SHARED_LIBS)
    message(FATAL_ERROR
      "FOLLY_BUILD_RUST_BINDINGS currently requires BUILD_SHARED_LIBS=ON. "
      "Cargo bridge objects need Folly's complete C++ dependency closure; "
      "exporting compiler-private rlibs as CMake static ABI targets is unsafe.")
  endif()
  if(NOT TARGET folly)
    message(FATAL_ERROR "Folly's C++ target must exist before enabling Rust")
  endif()

  find_program(FOLLY_RUSTUP_EXECUTABLE rustup
    HINTS "$ENV{HOME}/.cargo/bin")
  if(NOT FOLLY_RUSTUP_EXECUTABLE)
    message(FATAL_ERROR
      "FOLLY_BUILD_RUST_BINDINGS requires rustup; install the selected "
      "toolchain before configuring CMake")
  endif()

  if(NOT FOLLY_RUST_CARGO_PROFILE MATCHES "^(debug|release)$")
    message(FATAL_ERROR
      "FOLLY_RUST_CARGO_PROFILE must be debug or release, not "
      "'${FOLLY_RUST_CARGO_PROFILE}'")
  endif()
  if(FOLLY_RUST_CARGO_PROFILE STREQUAL "release")
    set(_folly_rust_profile_args --release)
  else()
    set(_folly_rust_profile_args)
  endif()

  if(WIN32)
    set(_folly_rust_path_separator ";")
  else()
    set(_folly_rust_path_separator ":")
  endif()

  set(_folly_rust_workspace "${CMAKE_CURRENT_SOURCE_DIR}/folly/rust")
  set(_folly_rust_target_dir "${CMAKE_CURRENT_BINARY_DIR}/folly-rust-target")
  set(_folly_rust_include_dirs
    "$<JOIN:$<TARGET_PROPERTY:folly,INCLUDE_DIRECTORIES>,${_folly_rust_path_separator}>")
  set(_folly_rust_cxxflags "$ENV{CXXFLAGS}")
  set(_folly_rust_bindgen_flags "$ENV{BINDGEN_EXTRA_CLANG_ARGS}")
  set(_folly_rustflags "$ENV{RUSTFLAGS}")

  foreach(_folly_rust_compile_flag IN LISTS FOLLY_CXX_RUNTIME_COMPILE_FLAGS)
    string(APPEND _folly_rust_cxxflags " ${_folly_rust_compile_flag}")
    string(APPEND _folly_rust_bindgen_flags " ${_folly_rust_compile_flag}")
  endforeach()

  string(APPEND _folly_rustflags
    " -C linker=${CMAKE_CXX_COMPILER}"
    " -C link-arg=-Wl,-rpath,$<TARGET_FILE_DIR:folly>")
  if(CMAKE_LINKER_TYPE STREQUAL "LLD")
    string(APPEND _folly_rustflags " -C link-arg=-fuse-ld=lld")
  endif()

  set(_folly_rust_cmake_link_flags "${CMAKE_EXE_LINKER_FLAGS}")
  string(TOUPPER "${CMAKE_BUILD_TYPE}" _folly_rust_build_type)
  if(_folly_rust_build_type)
    string(APPEND _folly_rust_cmake_link_flags
      " ${CMAKE_EXE_LINKER_FLAGS_${_folly_rust_build_type}}")
  endif()
  separate_arguments(_folly_rust_cmake_link_flags NATIVE_COMMAND
    "${_folly_rust_cmake_link_flags}")
  foreach(_folly_rust_link_flag IN LISTS _folly_rust_cmake_link_flags)
    string(APPEND _folly_rustflags
      " -C link-arg=${_folly_rust_link_flag}")
  endforeach()

  if(TARGET Jemalloc::jemalloc)
    set(_folly_rust_use_jemalloc 1)
    set(_folly_rust_jemalloc_library_dir
      "$<TARGET_FILE_DIR:Jemalloc::jemalloc>")
    string(APPEND _folly_rustflags
      " -C link-arg=-Wl,-rpath,$<TARGET_FILE_DIR:Jemalloc::jemalloc>")
  else()
    set(_folly_rust_use_jemalloc 0)
    set(_folly_rust_jemalloc_library_dir "")
  endif()

  foreach(_folly_rust_link_flag IN LISTS FOLLY_CXX_RUNTIME_LINK_FLAGS)
    string(APPEND _folly_rustflags
      " -C link-arg=${_folly_rust_link_flag}")
  endforeach()

  if(APPLE)
    string(APPEND _folly_rust_cxxflags
      " -isysroot ${CMAKE_OSX_SYSROOT}"
      " -mmacosx-version-min=${CMAKE_OSX_DEPLOYMENT_TARGET}")
    string(APPEND _folly_rust_bindgen_flags
      " -isysroot ${CMAKE_OSX_SYSROOT}"
      " -mmacosx-version-min=${CMAKE_OSX_DEPLOYMENT_TARGET}")
  endif()

  set(_folly_rust_environment
    "CARGO_TARGET_DIR=${_folly_rust_target_dir}"
    "CC=${CMAKE_C_COMPILER}"
    "CXX=${CMAKE_CXX_COMPILER}"
    "CXXFLAGS=${_folly_rust_cxxflags}"
    "FOLLY_RUST_FOLLY_INCLUDE_DIRS=${_folly_rust_include_dirs}"
    "FOLLY_RUST_FOLLY_LIBRARY_DIR=$<TARGET_FILE_DIR:folly>"
    "FOLLY_RUST_USE_JEMALLOC=${_folly_rust_use_jemalloc}"
    "FOLLY_RUST_JEMALLOC_LIBRARY_DIR=${_folly_rust_jemalloc_library_dir}"
    "RUSTFLAGS=${_folly_rustflags}"
  )
  if(APPLE)
    list(APPEND _folly_rust_environment
      "SDKROOT=${CMAKE_OSX_SYSROOT}"
      "MACOSX_DEPLOYMENT_TARGET=${CMAKE_OSX_DEPLOYMENT_TARGET}"
      "BINDGEN_EXTRA_CLANG_ARGS=${_folly_rust_bindgen_flags}"
    )
  endif()

  set(_folly_cargo_command
    "${CMAKE_COMMAND}" -E env
    ${_folly_rust_environment}
    "${FOLLY_RUSTUP_EXECUTABLE}" run "${FOLLY_RUST_TOOLCHAIN}"
    cargo test
    ${_folly_rust_profile_args}
    --workspace --locked
  )

  add_custom_target(folly_rust_bindings ALL
    COMMAND ${_folly_cargo_command} --no-run
    DEPENDS folly
    WORKING_DIRECTORY "${_folly_rust_workspace}"
    COMMENT
      "Building Folly Rust workspace with ${FOLLY_RUST_TOOLCHAIN} "
      "(${FOLLY_RUST_CARGO_PROFILE})"
    VERBATIM
    USES_TERMINAL
  )

  if(BUILD_TESTS)
    add_test(
      NAME folly_rust_workspace
      COMMAND ${_folly_cargo_command}
      WORKING_DIRECTORY "${_folly_rust_workspace}"
    )
    set_tests_properties(folly_rust_workspace PROPERTIES
      LABELS rust
      RUN_SERIAL TRUE
      TIMEOUT 300
    )
  endif()

  install(
    DIRECTORY "${_folly_rust_workspace}/"
    DESTINATION "${CMAKE_INSTALL_DATADIR}/folly/rust"
    COMPONENT dev
    PATTERN "target" EXCLUDE
    PATTERN "BUCK" EXCLUDE
  )
endfunction()
