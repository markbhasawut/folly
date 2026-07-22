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

folly_define_python_tests(
  CPP_TESTS
    AsyncGeneratorTest
    ErrorTest
    IOBufExtTest
    WeakTest
  PYTHON_TESTS
    coro.py
    futures.py
    generator.py
    iobuf.py
    request_context.py
    teardown.py
    test_iobuf_ext.py
    test_set_executor.py
)
