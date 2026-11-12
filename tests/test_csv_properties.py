"""CSV comparisons with MuleSoft's playground, checked 2026-11-12."""
import pytest

from dwpy.formats import FormatRegistry
from dwpy.runtime import DataWeaveRuntime


@pytest.mark.parametrize("backend", ["python", "rust"])
def test_csv_writer_properties_match_playground(backend):
    runtime = DataWeaveRuntime(backend=backend)
    script = r'''%dw 2.0
output application/csv quoteHeader=true quoteValues=true lineSeparator="\r\n"
---
payload'''
    result = runtime.execute(script, [{"a": "a,b", "b": 'a"b', "c": "x\\y"}])
    assert result == '"a","b","c"\r\n"a,b","a\\"b","x\\\\y"\r\n'


@pytest.mark.parametrize("backend", ["python", "rust"])
def test_csv_payload_reader_properties_and_multiline(backend):
    runtime = DataWeaveRuntime(backend=backend)
    result = runtime.execute('%dw 2.0\noutput application/python\n---\npayload',
        'a;b\n"hello\nworld";a!;b\n\n ;z\n', payload_format="csv",
        payload_format_options={"separator": ";", "escape": "!", "ignoreEmptyLine": False})
    assert result == [{"a": "hello\nworld", "b": "a;b"}, {"a": ""}, {"a": " ", "b": "z"}]


def test_csv_default_writer_escapes_special_values():
    assert FormatRegistry.write([{"a": "a,b", "b": 'a"b', "c": "x\\y"}], "csv", {}) == 'a,b,c\na\\,b,a\\"b,x\\\\y\n'


def test_csv_reader_retains_irregular_columns_and_whitespace():
    assert FormatRegistry.read('a,b\n1\n2,3,4\n \n\n', "csv", {}) == [
        {"a": "1"}, {"a": "2", "b": "3", "column_2": "4"}]


def test_csv_custom_escape_and_no_header():
    assert FormatRegistry.write([{"a": "a,b", "b": 'a"b'}], "csv", {"escape": "!", "header": False}) == 'a!,b,a!"b\n'
    assert FormatRegistry.read('a!,b,c\n', "csv", {"escape": "!", "header": False}) == [["a,b", "c"]]


@pytest.mark.parametrize("backend", ["python", "rust"])
def test_csv_read_and_write_functions(backend):
    runtime = DataWeaveRuntime(backend=backend)
    script = r'''%dw 2.0
output application/python
---
{parsed: read("a,b\na!,b,c", "application/csv", {escape: "!"}), written: write([{a: "x,y"}], "application/csv", {quoteValues: true, lineSeparator: "|"})}'''
    assert runtime.execute(script, None) == {"parsed": [{"a": "a,b", "b": "c"}], "written": 'a|"x,y"|'}


@pytest.mark.parametrize("backend", ["python", "rust"])
def test_read_headerless_and_empty_quoted_cell(backend):
    runtime = DataWeaveRuntime(backend=backend)
    script = r'''%dw 2.0
output application/python
---
read("\"\";x\n \nhello;world", "application/csv", {header: false, separator: ";"})'''
    assert runtime.execute(script, None) == [{"column_0": "", "column_1": "x"}, {"column_0": "hello", "column_1": "world"}]


@pytest.mark.parametrize("backend", ["python", "rust"])
def test_csv_directive_line_separator_with_spaces(backend):
    runtime = DataWeaveRuntime(backend=backend)
    assert runtime.execute('%dw 2.0\noutput application/csv lineSeparator=" | "\n---\npayload', [{"a": 1}]) == 'a | 1 | '
