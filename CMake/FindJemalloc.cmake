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

include(FindPackageHandleStandardArgs)
find_package_handle_standard_args(
  Jemalloc
  REQUIRED_VARS Jemalloc_LIBRARY Jemalloc_INCLUDE_DIR
  VERSION_VAR Jemalloc_VERSION
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

mark_as_advanced(Jemalloc_INCLUDE_DIR Jemalloc_LIBRARY)
