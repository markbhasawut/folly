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

find_path(
  Jemalloc_INCLUDE_DIR
  NAMES jemalloc/jemalloc.h
)
find_library(
  Jemalloc_LIBRARY
  NAMES jemalloc
)

if(Jemalloc_INCLUDE_DIR)
  file(
    STRINGS
    "${Jemalloc_INCLUDE_DIR}/jemalloc/jemalloc.h"
    _jemalloc_version_line
    REGEX "^#define JEMALLOC_VERSION \"[^\"]+\""
    LIMIT_COUNT 1
  )
  string(
    REGEX REPLACE
    "^#define JEMALLOC_VERSION \"([^\"]+)\"$"
    "\\1"
    Jemalloc_VERSION
    "${_jemalloc_version_line}"
  )
endif()

if(Jemalloc_INCLUDE_DIR AND Jemalloc_LIBRARY)
  include(CheckCXXSourceCompiles)

  set(_jemalloc_saved_required_includes "${CMAKE_REQUIRED_INCLUDES}")
  set(_jemalloc_saved_required_libraries "${CMAKE_REQUIRED_LIBRARIES}")
  set(CMAKE_REQUIRED_INCLUDES "${Jemalloc_INCLUDE_DIR}")
  set(CMAKE_REQUIRED_LIBRARIES "${Jemalloc_LIBRARY}")

  # Folly calls jemalloc's public allocation API directly. On macOS, jemalloc
  # defaults to a je_ prefix unless configured with --with-jemalloc-prefix=.
  # Finding a library named libjemalloc is therefore insufficient: reject an
  # ABI that would leave mallocx, nallocx, dallocx, and mallctl unavailable.
  unset(Jemalloc_HAS_UNPREFIXED_API CACHE)
  check_cxx_source_compiles(
    [[
      #include <cstddef>
      #include <jemalloc/jemalloc.h>

      int main() {
        void* allocation = mallocx(8, MALLOCX_ZERO);
        const std::size_t capacity = nallocx(8, 0);
        dallocx(allocation, 0);

        const char* version = nullptr;
        std::size_t version_size = sizeof(version);
        const int error =
            mallctl("version", &version, &version_size, nullptr, 0);
        return error != 0 || capacity < 8;
      }
    ]]
    Jemalloc_HAS_UNPREFIXED_API
  )

  set(CMAKE_REQUIRED_INCLUDES "${_jemalloc_saved_required_includes}")
  set(CMAKE_REQUIRED_LIBRARIES "${_jemalloc_saved_required_libraries}")
  unset(_jemalloc_saved_required_includes)
  unset(_jemalloc_saved_required_libraries)
endif()

include(FindPackageHandleStandardArgs)
find_package_handle_standard_args(
  Jemalloc
  REQUIRED_VARS
    Jemalloc_LIBRARY
    Jemalloc_INCLUDE_DIR
    Jemalloc_HAS_UNPREFIXED_API
  VERSION_VAR Jemalloc_VERSION
  REASON_FAILURE_MESSAGE
    "Folly requires an unprefixed jemalloc ABI. Configure jemalloc with --with-jemalloc-prefix="
)

if(Jemalloc_FOUND AND NOT TARGET Jemalloc::jemalloc)
  add_library(Jemalloc::jemalloc UNKNOWN IMPORTED)
  set_target_properties(
    Jemalloc::jemalloc
    PROPERTIES
      IMPORTED_LOCATION "${Jemalloc_LIBRARY}"
      INTERFACE_INCLUDE_DIRECTORIES "${Jemalloc_INCLUDE_DIR}"
  )
endif()

# Compatibility variables for consumers that do not use imported targets.
set(JEMALLOC_FOUND "${Jemalloc_FOUND}")
set(JEMALLOC_INCLUDE_DIR "${Jemalloc_INCLUDE_DIR}")
set(JEMALLOC_LIBRARY "${Jemalloc_LIBRARY}")

mark_as_advanced(
  Jemalloc_INCLUDE_DIR
  Jemalloc_LIBRARY
  Jemalloc_HAS_UNPREFIXED_API
)
