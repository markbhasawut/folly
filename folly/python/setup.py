#!/usr/bin/env python3
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

# Do not call directly, use cmake
#
# Cython requires source files in a specific structure, the structure is
# created as tree of links to the real source files.

import argparse
import os
import sys

import Cython
from Cython.Build import cythonize
from Cython.Compiler import Options
from setuptools import Extension, setup


Options.fast_fail = True


def parse_build_mode(argv: list) -> tuple:
    """
    Parse command line arguments to determine build mode.

    Args:
        argv: Command line arguments (typically sys.argv)

    Returns:
        Tuple of (is_api_only, remaining_args)
    """
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--api-only", action="store_true")
    args, remaining = parser.parse_known_args(argv[1:])
    return args.api_only, [argv[0]] + remaining


is_api_only, remaining_argv = parse_build_mode(sys.argv)
# Restore sys.argv with remaining args so setuptools sees its arguments
# (e.g., "build_ext", "-f", "-I/path") when setup() is called
sys.argv = remaining_argv

if is_api_only:
    # Generate API headers only (no C++ compilation or linking)
    # Used by CMake to generate iobuf_api.h before building folly_python_cpp
    Cython.Compiler.Main.compile(
        "folly/iobuf.pyx",
        full_module_name="folly.iobuf",
        cplus=True,
        language_level=3,
    )
    Cython.Compiler.Main.compile(
        "folly/executor.pyx",
        full_module_name="folly.executor",
        cplus=True,
        language_level=3,
    )
else:
    extra_link_args = []
    ldflags_str = os.environ.get("LDFLAGS", "")
    if ldflags_str:
        import shlex

        extra_link_args.extend(shlex.split(ldflags_str))

    if sys.platform == "darwin":
        if "-undefined" not in ldflags_str:
            extra_link_args.extend(["-undefined", "dynamic_lookup"])

    exts = [
        Extension(
            "folly.executor",
            sources=["folly/executor.pyx", "folly/ProactorExecutor.cpp"],
            libraries=["folly_python_cpp", "folly", "glog"],
            language="c++",
            extra_compile_args=["-std=c++20", "-DGLOG_USE_GLOG_EXPORT"],
            extra_link_args=extra_link_args,
        ),
        Extension(
            "folly.iobuf",
            sources=["folly/iobuf.pyx", "folly/iobuf_ext.cpp"],
            libraries=["folly_python_cpp", "folly", "glog"],
            language="c++",
            extra_compile_args=["-std=c++20", "-DGLOG_USE_GLOG_EXPORT"],
            extra_link_args=extra_link_args,
        ),
        Extension(
            "folly.build_mode",
            sources=["folly/build_mode.pyx"],
            libraries=["folly_python_cpp", "folly", "glog"],
            language="c++",
            extra_compile_args=["-std=c++20", "-DGLOG_USE_GLOG_EXPORT"],
            extra_link_args=extra_link_args,
        ),
        Extension(
            "folly.fiber_manager",
            sources=["folly/fiber_manager.pyx", "folly/fibers.cpp"],
            libraries=["folly_python_cpp", "folly", "glog"],
            language="c++",
            extra_compile_args=["-std=c++20", "-DGLOG_USE_GLOG_EXPORT"],
            extra_link_args=extra_link_args,
        ),
        Extension(
            "folly.request_context",
            sources=["folly/request_context.pyx"],
            libraries=["folly_python_cpp", "folly", "glog"],
            language="c++",
            extra_compile_args=["-std=c++20", "-DGLOG_USE_GLOG_EXPORT"],
            extra_link_args=extra_link_args,
        ),
    ]

    if sys.platform == "win32":
        exts.append(
            Extension(
                "folly.windows.iocp",
                sources=["folly/windows/iocp.pyx"],
                libraries=["folly_python_cpp", "folly", "glog"],
                language="c++",
                extra_compile_args=["/std:c++20", "-DGLOG_USE_GLOG_EXPORT"],
                extra_link_args=extra_link_args,
            )
        )

    setup(
        name="folly",
        version="0.0.1",
        packages=["folly", "folly.python", "folly.windows"],
        package_data={
            "": [
                "*.pyi",
                "*.pxd",
                "*.pyx",
                "*.py",
                "*.h",
                "*.so",
                "*.dylib",
                "*.dll",
                "py.typed",
            ],
        },
        setup_requires=["cython"],
        zip_safe=False,
        ext_modules=cythonize(exts, compiler_directives={"language_level": 3}),
    )
