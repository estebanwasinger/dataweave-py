import os

import pytest

from dwpy.runtime import DataWeaveRuntime


pytestmark = pytest.mark.skipif(
    os.environ.get("DWPY_TEST_BACKEND") is not None,
    reason="backend parity is tested outside the forced-backend matrix",
)


@pytest.mark.parametrize("backend", ["python", "rust"])
def test_string_helpers_and_null_sizes_match_across_backends(backend):
    script = r'''%dw 2.0
import * from dw::core::Strings
output application/python
---
do {
  fun measure(value) = sizeOf(value)
  ---
  {
    camel: ["CUSTOMER", "customerName", "foo_BAR", "_FOO_BAR", "foo__bar", "__", null] map camelize($),
    capital: ["some-title: The begining", "customerName", "a--b", "  some-title  ", "foo__bar", "a*s_b’s", "", null] map capitalize($),
    sizes: [sizeOf(null), sizeOf(null as Number), sizeOf(null) default 42, measure("abc"), measure([])]
  }
}'''
    assert DataWeaveRuntime(backend=backend).execute(script, None) == {
        "camel": ["cUSTOMER", "customerName", "fooBAR", "fOOBAR", "fooBar", "", None],
        "capital": ["Some Title: The Begining", "Customer Name", "A  B", "  Some Title  ", "Foo  Bar", "A*S B’S", "", None],
        "sizes": [None, None, 42, 3, 0],
    }


def test_auto_fallback_preserves_string_and_null_size_results():
    runtime = DataWeaveRuntime(backend="auto")
    script = '''%dw 2.0
import * from dw::core::Strings
output application/python
---
[camelize("CUSTOMER"), capitalize("some-title"), sizeOf(null) default 42, isLegacySizeOfNumber()]'''

    assert runtime.execute(script, None) == ["cUSTOMER", "Some Title", 42, False]
    assert runtime._rust_runtime.last_execution_engine() == "python-legacy"
