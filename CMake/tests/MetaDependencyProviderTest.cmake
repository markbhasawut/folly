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

cmake_minimum_required(VERSION 3.24)

include("${CMAKE_CURRENT_LIST_DIR}/../MetaDependencyProvider.cmake")

set(_install_name "/usr/local/lib/libjemalloc.2.dylib")
string(CONCAT _otool_output
  "/stage/usr/local/lib/libjemalloc.2.dylib:\n"
  "  /usr/local/lib/libjemalloc.2.dylib "
  "(compatibility version 0.0.0, current version 0.0.0)\n"
  "  /usr/local/lib/libfolly.0.58.0-dev.dylib "
  "(compatibility version 0.0.0, current version 0.58.0)\n"
  "  /usr/lib/libSystem.B.dylib "
  "(compatibility version 1.0.0, current version 1359.0.0)\n"
)

_meta_extract_absolute_macho_dependencies(
  _dependencies "${_otool_output}" "${_install_name}")

set(_expected_dependencies
  "/usr/local/lib/libfolly.0.58.0-dev.dylib"
  "/usr/lib/libSystem.B.dylib"
)
if(NOT _dependencies STREQUAL _expected_dependencies)
  message(FATAL_ERROR
    "LC_ID_DYLIB was not excluded correctly.\n"
    "Expected: ${_expected_dependencies}\n"
    "Actual: ${_dependencies}")
endif()

_meta_get_staged_dependency_candidate(
  _staged_dependency
  "/usr/local/lib/libjemalloc.2.dylib"
  "/stage/usr/local"
)
if(NOT _staged_dependency STREQUAL
    "/stage/usr/local/lib/libjemalloc.2.dylib")
  message(FATAL_ERROR
    "DESTDIR dependency mapping failed: ${_staged_dependency}")
endif()

_meta_get_staged_dependency_candidate(
  _mismatched_dependency
  "/opt/homebrew/lib/libjemalloc.2.dylib"
  "/stage/usr/local"
)
if(_mismatched_dependency)
  message(FATAL_ERROR
    "An unrelated provider mapped into the staging prefix: "
    "${_mismatched_dependency}")
endif()
