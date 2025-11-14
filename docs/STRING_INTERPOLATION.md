# String Interpolation Feature

## Overview

String interpolation allows embedding DataWeave expressions directly within string literals using the `$(expression)` syntax. This feature was added to enable dynamic string construction with evaluated expressions.

## Syntax

```dataweave
"text $(expression) more text"
```

## Examples

### Basic Usage

```python
runtime.execute('''%dw 2.0
output application/json
---
"hello $(payload.message)"
''', {"message": "world"})
# Returns: "hello world"
```

### Multiple Interpolations

```python
runtime.execute('''%dw 2.0
output application/json
---
"$(payload.greeting) $(payload.name)!"
''', {"greeting": "Hello", "name": "Alice"})
# Returns: "Hello Alice!"
```

### Expressions

```python
runtime.execute('''%dw 2.0
output application/json
---
"Total: $(payload.price * payload.quantity)"
''', {"price": 10, "quantity": 3})
# Returns: "Total: 30"
```

### Nested Property Access

```python
runtime.execute('''%dw 2.0
output application/json
---
"User: $(payload.user.name)"
''', {"user": {"name": "Bob"}})
# Returns: "User: Bob"
```

### Function Calls

```python
runtime.execute('''%dw 2.0
output application/json
---
"Uppercase: $(upper(payload.text))"
''', {"text": "hello"})
# Returns: "Uppercase: HELLO"
```

### Complex Expressions

```python
runtime.execute('''%dw 2.0
output application/json
---
"Result: $((payload.a + payload.b) * 2)"
''', {"a": 5, "b": 3})
# Returns: "Result: 16"
```

## Type Conversions

The interpolation automatically converts values to strings:

- **Strings**: Used as-is
- **Numbers**: Converted to string representation (e.g., `42` → `"42"`)
- **Booleans**: Converted to `"true"` or `"false"`
- **Null/None**: Converted to empty string `""`
- **Objects/Arrays**: JSON-serialized

## Implementation Details

### Parser Changes

1. Added `InterpolatedString` AST node in `parser.py`
2. Modified string parsing to detect `$(...)` patterns
3. Added `_parse_interpolated_string()` method to parse embedded expressions
4. Handles nested parentheses correctly

### Runtime Changes

1. Added evaluation logic for `InterpolatedString` in `runtime.py`
2. Added `_to_string()` helper method for type conversions
3. Concatenates evaluated parts into final string

### Test Coverage

Created comprehensive test suite in `tests/test_string_interpolation.py` with 17 tests covering:
- Basic interpolation
- Multiple interpolations
- Expressions in interpolation
- Nested property access
- Function calls
- Edge cases (null values, booleans, nested parentheses)
- Regression tests (strings without interpolation)

## Limitations

- The `$(` sequence is always treated as the start of interpolation
- To include a literal `$(` in a string, use concatenation or escaping (if implemented)
- Interpolation expressions must be valid DataWeave expressions

## Future Enhancements

Potential future improvements:
- Escape sequences for literal `$(` 
- Support for format specifiers (e.g., `$(value:format)`)
- More sophisticated number/date formatting



