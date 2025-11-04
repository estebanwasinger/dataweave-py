"""Tests for string interpolation feature."""
import pytest
from dwpy.runtime import DataWeaveRuntime


@pytest.fixture
def runtime():
    return DataWeaveRuntime()


def test_basic_string_interpolation(runtime):
    """Test basic string interpolation with a simple variable."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"hello $(payload.message)"
""",
        {"message": "world"},
    )
    assert result == "hello world"


def test_multiple_interpolations(runtime):
    """Test multiple interpolations in a single string."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"$(payload.greeting) $(payload.name)!"
""",
        {"greeting": "Hello", "name": "Alice"},
    )
    assert result == "Hello Alice!"


def test_expression_in_interpolation(runtime):
    """Test using expressions inside interpolation."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"Total: $(payload.price * payload.quantity)"
""",
        {"price": 10, "quantity": 3},
    )
    assert result == "Total: 30"


def test_nested_property_access(runtime):
    """Test nested property access in interpolation."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"User: $(payload.user.name)"
""",
        {"user": {"name": "Bob"}},
    )
    assert result == "User: Bob"


def test_function_call_in_interpolation(runtime):
    """Test calling functions inside interpolation."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"Uppercase: $(upper(payload.text))"
""",
        {"text": "hello"},
    )
    assert result == "Uppercase: HELLO"


def test_mixed_text_and_interpolations(runtime):
    """Test mixing literal text with multiple interpolations."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"The $(payload.animal) $(payload.action) over the $(payload.target)."
""",
        {"animal": "fox", "action": "jumps", "target": "fence"},
    )
    assert result == "The fox jumps over the fence."


def test_string_without_interpolation(runtime):
    """Test that regular strings still work (regression test)."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"Hello World"
""",
        {},
    )
    assert result == "Hello World"


def test_null_value_in_interpolation(runtime):
    """Test that null values in interpolation become empty strings."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"Value: $(payload.missing)"
""",
        {},
    )
    assert result == "Value: "


def test_nested_parentheses_in_expression(runtime):
    """Test handling nested parentheses in interpolated expressions."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"Result: $((payload.a + payload.b) * 2)"
""",
        {"a": 5, "b": 3},
    )
    assert result == "Result: 16"


def test_boolean_value_in_interpolation(runtime):
    """Test that boolean values are converted to 'true'/'false' strings."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"Active: $(payload.active)"
""",
        {"active": True},
    )
    assert result == "Active: true"


def test_number_value_in_interpolation(runtime):
    """Test that numbers are converted to strings."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"Count: $(payload.count)"
""",
        {"count": 42},
    )
    assert result == "Count: 42"


def test_interpolation_at_start_and_end(runtime):
    """Test interpolations at the beginning and end of strings."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"$(payload.prefix)-middle-$(payload.suffix)"
""",
        {"prefix": "start", "suffix": "end"},
    )
    assert result == "start-middle-end"


def test_only_interpolation_no_literal_text(runtime):
    """Test a string that is only an interpolation with no literal text."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"$(payload.text)"
""",
        {"text": "only-interpolation"},
    )
    assert result == "only-interpolation"


def test_interpolation_with_default_operator(runtime):
    """Test interpolation with the default operator."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"Hello $(payload.name default 'Guest')"
""",
        {},
    )
    assert result == "Hello Guest"


def test_interpolation_in_object_field(runtime):
    """Test using string interpolation in object fields."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
{
  message: "Hello $(payload.name)"
}
""",
        {"name": "World"},
    )
    assert result == {"message": "Hello World"}


def test_interpolation_with_concatenation(runtime):
    """Test interpolation with string concatenation."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"Result: $(payload.first ++ ' ' ++ payload.last)"
""",
        {"first": "John", "last": "Doe"},
    )
    assert result == "Result: John Doe"


def test_float_value_in_interpolation(runtime):
    """Test that float values are converted to strings properly."""
    result = runtime.execute(
        """%dw 2.0
output application/json
---
"Price: $(payload.price)"
""",
        {"price": 19.99},
    )
    assert result == "Price: 19.99"

