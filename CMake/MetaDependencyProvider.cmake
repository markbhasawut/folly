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

include_guard(GLOBAL)

if(APPLE)
  set(_META_SINGLE_DEPENDENCY_PREFIX_DEFAULT ON)
else()
  set(_META_SINGLE_DEPENDENCY_PREFIX_DEFAULT OFF)
endif()

option(
  META_ENFORCE_SINGLE_DEPENDENCY_PREFIX
  "Require the Meta C++ stack and jemalloc to come from one install prefix"
  ${_META_SINGLE_DEPENDENCY_PREFIX_DEFAULT}
)
set(
  META_DEPENDENCY_PREFIX
  ""
  CACHE PATH
  "Expected install prefix for jemalloc, Folly, Fizz, Wangle, mvfst, Proxygen, FBThrift, and fb303"
)

function(_meta_get_imported_location _output _target)
  if(NOT TARGET "${_target}")
    set("${_output}" "" PARENT_SCOPE)
    return()
  endif()

  get_target_property(_aliased_target "${_target}" ALIASED_TARGET)
  if(_aliased_target)
    set(_target "${_aliased_target}")
  endif()

  get_target_property(_imported "${_target}" IMPORTED)
  if(NOT _imported)
    set("${_output}" "" PARENT_SCOPE)
    return()
  endif()

  set(_properties IMPORTED_LOCATION)
  if(CMAKE_BUILD_TYPE)
    string(TOUPPER "${CMAKE_BUILD_TYPE}" _build_type)
    list(PREPEND _properties "IMPORTED_LOCATION_${_build_type}")
  endif()

  get_target_property(_configurations "${_target}" IMPORTED_CONFIGURATIONS)
  foreach(_configuration IN LISTS _configurations)
    string(TOUPPER "${_configuration}" _configuration)
    list(APPEND _properties "IMPORTED_LOCATION_${_configuration}")
  endforeach()

  foreach(_property IN LISTS _properties)
    get_target_property(_location "${_target}" "${_property}")
    if(_location AND NOT _location MATCHES "-NOTFOUND$")
      set("${_output}" "${_location}" PARENT_SCOPE)
      return()
    endif()
  endforeach()

  set("${_output}" "" PARENT_SCOPE)
endfunction()

function(_meta_path_is_under_prefix _output _path _prefix)
  file(TO_CMAKE_PATH "${_path}" _path)
  file(TO_CMAKE_PATH "${_prefix}" _prefix)
  string(REGEX REPLACE "/+$" "" _path "${_path}")
  string(REGEX REPLACE "/+$" "" _prefix "${_prefix}")
  string(FIND "${_path}/" "${_prefix}/" _prefix_position)
  if(_prefix_position EQUAL 0)
    set("${_output}" TRUE PARENT_SCOPE)
  else()
    set("${_output}" FALSE PARENT_SCOPE)
  endif()
endfunction()

function(_meta_is_stack_dylib _output _path)
  get_filename_component(_name "${_path}" NAME)
  if(_name MATCHES
      "^lib(jemalloc|folly|fizz|wangle|mvfst|proxygen|fbthrift|fb303|thrift).*\\.dylib$")
    set("${_output}" TRUE PARENT_SCOPE)
  else()
    set("${_output}" FALSE PARENT_SCOPE)
  endif()
endfunction()

function(_meta_validate_macho_dependencies _binary _prefix _package)
  if(NOT APPLE OR NOT EXISTS "${_binary}")
    return()
  endif()

  execute_process(
    COMMAND /usr/bin/otool -L "${_binary}"
    RESULT_VARIABLE _otool_result
    OUTPUT_VARIABLE _otool_output
    ERROR_VARIABLE _otool_error
  )
  if(NOT _otool_result EQUAL 0)
    message(FATAL_ERROR
      "${_package}: unable to inspect ${_binary} with otool:\n${_otool_error}")
  endif()

  string(REPLACE "\n" ";" _otool_lines "${_otool_output}")
  foreach(_line IN LISTS _otool_lines)
    string(STRIP "${_line}" _line)
    if(NOT _line MATCHES "^(/[^ ]+)[ \t]+\\(compatibility version")
      continue()
    endif()

    set(_dependency "${CMAKE_MATCH_1}")
    _meta_is_stack_dylib(_is_meta_stack_dylib "${_dependency}")
    if(NOT _is_meta_stack_dylib)
      continue()
    endif()

    _meta_path_is_under_prefix(
      _dependency_matches_prefix "${_dependency}" "${_prefix}")
    if(NOT _dependency_matches_prefix)
      message(FATAL_ERROR
        "${_package}: mixed Meta dependency providers detected.\n"
        "  Binary: ${_binary}\n"
        "  Embedded dependency: ${_dependency}\n"
        "  Required prefix: ${_prefix}\n"
        "Rebuild and reinstall the offending dependency from the selected "
        "prefix, then configure from a clean build directory. Set "
        "META_DEPENDENCY_PREFIX explicitly only when auto-detection is "
        "unsuitable.")
    endif()
  endforeach()
endfunction()

function(meta_validate_dependency_provider)
  if(NOT META_ENFORCE_SINGLE_DEPENDENCY_PREFIX)
    return()
  endif()

  cmake_parse_arguments(
    PARSE_ARGV 0
    META_PROVIDER
    ""
    "ANCHOR_TARGET;PACKAGE"
    "TARGETS"
  )
  if(META_PROVIDER_PACKAGE)
    set(_package "${META_PROVIDER_PACKAGE}")
  else()
    set(_package "Meta C++ dependency stack")
  endif()

  if(META_DEPENDENCY_PREFIX)
    get_filename_component(
      _required_prefix "${META_DEPENDENCY_PREFIX}" ABSOLUTE)
  else()
    if(META_PROVIDER_ANCHOR_TARGET)
      set(_anchor_target "${META_PROVIDER_ANCHOR_TARGET}")
    else()
      set(_anchor_target Folly::folly)
    endif()
    _meta_get_imported_location(_anchor_location "${_anchor_target}")
    if(NOT _anchor_location)
      message(STATUS
        "${_package}: single-provider validation skipped because "
        "${_anchor_target} is built in-tree or has no imported location")
      return()
    endif()
    get_filename_component(_anchor_library_dir "${_anchor_location}" DIRECTORY)
    get_filename_component(_required_prefix "${_anchor_library_dir}" DIRECTORY)
  endif()
  get_filename_component(_required_prefix "${_required_prefix}" REALPATH)

  set(_checked_binaries)
  foreach(_target IN LISTS META_PROVIDER_TARGETS)
    if(NOT TARGET "${_target}")
      message(FATAL_ERROR
        "${_package}: expected dependency target ${_target}, but it does not "
        "exist")
    endif()

    _meta_get_imported_location(_location "${_target}")
    if(NOT _location)
      continue()
    endif()
    get_filename_component(_location "${_location}" REALPATH)
    _meta_path_is_under_prefix(
      _location_matches_prefix "${_location}" "${_required_prefix}")
    if(NOT _location_matches_prefix)
      message(FATAL_ERROR
        "${_package}: mixed Meta dependency providers detected.\n"
        "  Target: ${_target}\n"
        "  Imported library: ${_location}\n"
        "  Required prefix: ${_required_prefix}\n"
        "Use one install prefix for jemalloc, Folly, Fizz, Wangle, mvfst, "
        "Proxygen, FBThrift, and fb303. Homebrew remains supported for "
        "unrelated third-party dependencies.")
    endif()

    if(NOT _location IN_LIST _checked_binaries)
      list(APPEND _checked_binaries "${_location}")
      _meta_validate_macho_dependencies(
        "${_location}" "${_required_prefix}" "${_package}")
    endif()
  endforeach()

  list(LENGTH _checked_binaries _checked_binary_count)
  message(STATUS
    "${_package}: validated ${_checked_binary_count} imported libraries "
    "from ${_required_prefix}")
endfunction()
