import pytest

from dwpy import builtins
from dwpy.runtime import DataWeaveRuntime


def test_strings_module_exports_all_documented_functions():
    documented = {
        "appendIfMissing",
        "camelize",
        "capitalize",
        "charCode",
        "charCodeAt",
        "collapse",
        "countCharactersBy",
        "countMatches",
        "dasherize",
        "everyCharacter",
        "first",
        "fromCharCode",
        "hammingDistance",
        "isAlpha",
        "isAlphanumeric",
        "isLowerCase",
        "isNumeric",
        "isUpperCase",
        "isWhitespace",
        "last",
        "leftPad",
        "levenshteinDistance",
        "lines",
        "mapString",
        "ordinalize",
        "pluralize",
        "prependIfMissing",
        "remove",
        "repeat",
        "replaceAll",
        "reverse",
        "rightPad",
        "singularize",
        "someCharacter",
        "substring",
        "substringAfter",
        "substringAfterLast",
        "substringBefore",
        "substringBeforeLast",
        "substringBy",
        "substringEvery",
        "underscore",
        "unwrap",
        "withMaxSize",
        "words",
        "wrapIfMissing",
        "wrapWith",
    }

    exports = builtins.resolve_module_exports("dw::core::Strings")
    assert documented.issubset(exports.keys())
    assert all(callable(exports[name]) for name in documented)


def test_documented_string_functions_examples():
    assert builtins.builtin_append_if_missing(None, "") is None
    assert builtins.builtin_append_if_missing("abc", "xyz") == "abcxyz"
    assert builtins.builtin_append_if_missing("abcxyz", "xyz") == "abcxyz"

    assert builtins.builtin_camelize("customer_first_name") == "customerFirstName"
    assert builtins.builtin_capitalize("customerName") == "Customer Name"

    assert builtins.builtin_char_code("Mule") == 77
    assert builtins.builtin_char_code_at("MuleSoft", 1) == 117
    with pytest.raises(ValueError):
        builtins.builtin_char_code_at("abc", 99)

    assert builtins.builtin_collapse("a  b babb a") == ["a", "  ", "b", " ", "b", "a", "bb", " ", "a"]
    assert builtins.builtin_count_characters_by(
        "42 = 11 * 2 + 20",
        lambda char, index=None: builtins.builtin_is_numeric(char),
    ) == 7
    assert builtins.builtin_count_matches("hello worlo!", "lo") == 2
    assert builtins.builtin_count_matches("hello, ciao!", "/[aeiou]/") == 5

    assert builtins.builtin_dasherize("customer_first_name") == "customer-first-name"
    assert builtins.builtin_every_character("12 34  56", lambda ch, idx=None: ch == " " or ch.isdigit())
    assert builtins.builtin_first("hello world!", 5) == "hello"
    assert builtins.builtin_first("hello world!", 5.9) == "hello"
    assert builtins.builtin_from_char_code(117) == "u"

    assert builtins.builtin_hamming_distance("holu", "chau") == 3
    assert builtins.builtin_hamming_distance("abc", "ab") is None

    assert builtins.builtin_is_alpha("abc") is True
    assert builtins.builtin_is_alphanumeric("ab2c") is True
    assert builtins.builtin_is_lower_case("mulesoft") is True
    assert builtins.builtin_is_numeric("१२३") is True
    assert builtins.builtin_is_upper_case("ABC") is True
    assert builtins.builtin_is_whitespace("") is True

    assert builtins.builtin_last("hello world!", 6) == "world!"
    assert builtins.builtin_last("hello world!", 5.1) == "world!"
    assert builtins.builtin_left_pad("bat", 5) == "  bat"
    assert builtins.builtin_right_pad("bat", 5) == "bat  "
    assert builtins.builtin_levenshtein_distance("kitten", "sitting") == 3
    assert builtins.builtin_lines("hello world\n\nhere   data-weave") == ["hello world", "", "here   data-weave"]

    assert builtins.builtin_map_string("$234", lambda ch, idx=None: "~" if ch.isdigit() else ch) == "$~~~"
    assert builtins.builtin_ordinalize(103) == "103rd"
    assert builtins.builtin_pluralize("box") == "boxes"
    assert builtins.builtin_singularize("boxes") == "box"
    assert builtins.builtin_prepend_if_missing("abc", "xyz") == "xyzabc"
    assert builtins.builtin_remove("lazyness purity state higher-order stateful", "state") == "lazyness purity  higher-order ful"
    assert builtins.builtin_repeat("e", 3) == "eee"
    assert builtins.builtin_replace_all("AAAA", "AAA", "B") == "BA"
    assert builtins.builtin_reverse("Mariano") == "onairaM"
    assert builtins.builtin_some_character("someCharacter", lambda ch, idx=None: ch.isupper())

    assert builtins.builtin_substring("hello world!", 1, 5) == "ello"
    assert builtins.builtin_substring_after("abcba", "b") == "cba"
    assert builtins.builtin_substring_after("abc", "") == "abc"
    assert builtins.builtin_substring_after_last("abcba", "b") == "a"
    assert builtins.builtin_substring_after_last("abc", "") is None
    assert builtins.builtin_substring_before("abc", "c") == "ab"
    assert builtins.builtin_substring_before("abc", "") == ""
    assert builtins.builtin_substring_before_last("abcba", "b") == "abc"
    assert builtins.builtin_substring_before_last("abc", "") == "ab"
    assert builtins.builtin_substring_by(
        "hello~world=here_data-weave",
        lambda ch, idx=None: ch in {"~", "=", "_"},
    ) == ["hello", "world", "here", "data-weave"]
    assert builtins.builtin_substring_every("substringEvery", 3) == ["sub", "str", "ing", "Eve", "ry"]

    assert builtins.builtin_underscore("customerName") == "customer_name"
    assert builtins.builtin_unwrap("'abc'", "'") == "abc"
    assert builtins.builtin_unwrap("#A", "#") == "#A"
    assert builtins.builtin_with_max_size("123", 2) == "12"
    assert builtins.builtin_with_max_size("123", 0) == "123"
    assert builtins.builtin_words("hello world\nhere\t\t\tdata-weave") == ["hello", "world", "here", "data-weave"]

    assert builtins.builtin_wrap_if_missing("", "'") == "'"
    assert builtins.builtin_wrap_if_missing("a/b/c", "/") == "/a/b/c/"
    assert builtins.builtin_wrap_with("ab", "'") == "'ab'"


def test_strings_functions_work_through_module_import():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
import * from dw::core::Strings
---
{
  append: appendIfMissing("abc", "xyz"),
  dashed: dasherize("customerName"),
  hamming: hammingDistance("holu", "chau"),
  sliced: substringBeforeLast("abcba", "b"),
  wrapped: wrapWith("ab", "'")
}
"""
    result = runtime.execute(script, payload={}, render_output=False)

    assert result == {
        "append": "abcxyz",
        "dashed": "customer-name",
        "hamming": 3,
        "sliced": "abc",
        "wrapped": "'ab'",
    }
