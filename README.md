# DataWeave-Py

A native Python implementation of the DataWeave data transformation language, providing powerful data transformation capabilities directly in Python without requiring the JVM.

## Overview

DataWeave-Py (`dwpy`) is a Python interpreter for the DataWeave language, originally developed by MuleSoft for data transformation in the Mule runtime. This project brings DataWeave's expressive transformation syntax and rich feature set to the Python ecosystem, enabling:

- **Data transformation**: Convert between JSON, XML, CSV and other formats
- **Functional programming**: Leverage map, filter, reduce, and other functional operators
- **Pattern matching**: Use powerful match expressions with guards and bindings
- **Safe navigation**: Handle null values gracefully with null-safe operators
- **Rich built-ins**: Access 100+ built-in functions for strings, numbers, dates, arrays, and objects

## Requirements

- Python 3.10 or higher
- Dependencies managed via [uv](https://github.com/astral-sh/uv) (recommended) or pip

## Quick Start

### Basic Usage

```python
from dwpy import DataWeaveRuntime

# Create a runtime instance
runtime = DataWeaveRuntime()

# Define a DataWeave script
script = """%dw 2.0
output application/json
---
{
  message: "Hello, " ++ upper(payload.name),
  timestamp: now()
}
"""

# Execute with a payload
payload = {"name": "world"}
result = runtime.execute(script, payload)

print(result)
# Output: {'message': 'Hello, WORLD', 'timestamp': '2025-11-03T...Z'}
```

### Data Transformation Example

```python
from dwpy import DataWeaveRuntime

runtime = DataWeaveRuntime()

# Transform and enrich order data
script = """%dw 2.0
output application/json
---
{
  orderId: payload.id,
  status: upper(payload.status default "pending"),
  total: payload.items reduce ((item, acc = 0) -> 
    acc + (item.price * (item.quantity default 1))
  ),
  itemCount: sizeOf(payload.items)
}
"""

payload = {
    "id": "ORD-123",
    "status": "confirmed",
    "items": [
        {"price": 29.99, "quantity": 2},
        {"price": 15.50, "quantity": 1}
    ]
}

result = runtime.execute(script, payload)
print(result)
# Output: {'orderId': 'ORD-123', 'status': 'CONFIRMED', 'total': 75.48, 'itemCount': 2}
```

### Using Variables

```python
from dwpy import DataWeaveRuntime

runtime = DataWeaveRuntime()

script = """%dw 2.0
output application/json
var requestTime = vars.requestTime default now()
---
{
  user: payload.userId,
  processedAt: requestTime
}
"""

payload = {"userId": "U-456"}
vars = {"requestTime": "2024-05-05T12:00:00Z"}

result = runtime.execute(script, payload, vars=vars)
```

### Pattern Matching

```python
from dwpy import DataWeaveRuntime

runtime = DataWeaveRuntime()

script = """%dw 2.0
output application/json
---
{
  category: payload.price match {
    case var p when p > 100 -> "premium",
    case var p when p > 50 -> "standard",
    else -> "budget"
  }
}
"""

result = runtime.execute(script, {"price": 75})
# Output: {'category': 'standard'}
```

### String Interpolation

```python
from dwpy import DataWeaveRuntime

runtime = DataWeaveRuntime()

# Simple interpolation
script = """%dw 2.0
output application/json
---
{
  greeting: "Hello $(payload.name)!",
  total: "Total: $(payload.price * payload.quantity)",
  status: "Order $(payload.orderId) is $(upper(payload.status))"
}
"""

payload = {
    "name": "Alice",
    "price": 10.5,
    "quantity": 3,
    "orderId": "ORD-123",
    "status": "confirmed"
}

result = runtime.execute(script, payload)
# Output: {
#   'greeting': 'Hello Alice!',
#   'total': 'Total: 31.5',
#   'status': 'Order ORD-123 is CONFIRMED'
# }
```

String interpolation allows you to embed expressions directly within strings using the `$(expression)` syntax. The expression can be:
- Property access: `$(payload.name)`
- Nested properties: `$(payload.user.email)`
- Expressions: `$(payload.price * 1.1)`
- Function calls: `$(upper(payload.status))`
- Any valid DataWeave expression

## Supported Features

DataWeave-Py currently supports a wide range of DataWeave language features:

### Core Language Features
- ✅ Header directives (`%dw 2.0`, `output`, `var`, `import`)
- ✅ Payload and variable access
- ✅ Object and array literals
- ✅ Field selectors (`.field`, `?.field`, `[index]`)
- ✅ Comments (line `//` and block `/* */`)
- ✅ Default values (`payload.field default "fallback"`)
- ✅ String interpolation (`"Hello $(payload.name)"`)

### Operators
- ✅ Concatenation (`++`)
- ✅ Difference (`--`)
- ✅ Arithmetic (`+`, `-`, `*`, `/`)
- ✅ Comparison (`==`, `!=`, `>`, `<`, `>=`, `<=`)
- ✅ Logical (`and`, `or`, `not`)
- ✅ Range (`to`)

### Control Flow
- ✅ Conditional expressions (`if-else`)
- ✅ Pattern matching (`match-case`)
- ✅ Match guards (`case var x when condition`)

### Collection Operations
- ✅ `map` - Transform elements
- ✅ `filter` - Select elements
- ✅ `reduce` - Aggregate values
- ✅ `flatMap` - Map and flatten
- ✅ `distinctBy` - Remove duplicates
- ✅ `groupBy` - Group by criteria
- ✅ `orderBy` - Sort elements

### Built-in Functions

#### String Functions
`upper`, `lower`, `trim`, `contains`, `startsWith`, `endsWith`, `isBlank`, `splitBy`, `joinBy`, `find`, `match`, `matches`

#### Numeric Functions
`abs`, `ceil`, `floor`, `round`, `pow`, `mod`, `sum`, `avg`, `max`, `min`, `random`, `randomInt`, `isDecimal`, `isInteger`, `isEven`, `isOdd`

#### Array/Object Functions
`sizeOf`, `isEmpty`, `flatten`, `indexOf`, `lastIndexOf`, `distinctBy`, `filterObject`, `keysOf`, `valuesOf`, `entriesOf`, `pluck`, `maxBy`, `minBy`

#### Date Functions
`now`, `isLeapYear`, `daysBetween`

#### Utility Functions
`log`, `logInfo`, `logDebug`, `logWarn`, `logError`

## Running Tests

The project includes comprehensive test coverage:

```bash
# Run all tests
pytest

# Run specific test file
pytest tests/test_runtime_basic.py

# Run with verbose output
pytest -v

# Run with coverage
pytest --cov=dwpy
```

## Project Structure

```
runtime-2.11.0-20250825-src/
├── dwpy/                      # Main Python package
│   ├── __init__.py           # Package exports
│   ├── parser.py             # DataWeave parser
│   ├── runtime.py            # Execution engine
│   └── builtins.py           # Built-in functions
├── tests/                     # Test suite
│   ├── test_runtime_basic.py # Core functionality tests
│   ├── test_builtins.py      # Built-in function tests
│   └── fixtures/             # Test data and fixtures
├── runtime-2.11.0-20250825/  # Original JVM runtime reference
├── docs/                      # Documentation
├── pyproject.toml            # Project configuration
└── README.md                 # This file
```

## Development

### Setting Up Development Environment

```bash
# Create virtual environment
uv venv --python 3.12
source .venv/bin/activate

# Install development dependencies
uv pip sync

# Install in editable mode
pip install -e .
```

### Running the Test Suite

```bash
# Run all tests
python -m pytest tests/

# Run specific test category
python -m pytest tests/test_builtins.py

# Run with coverage report
python -m pytest --cov=dwpy --cov-report=html tests/
```

### Code Style

The project follows standard Python conventions:
- PEP 8 style guide
- Type hints where appropriate
- Comprehensive docstrings
- Two-space indentation for consistency with Scala codebase

## Comparison with JVM Runtime

DataWeave-Py aims to provide feature parity with the official JVM-based DataWeave runtime. Key differences:

| Feature | JVM Runtime | DataWeave-Py |
|---------|-------------|--------------|
| Language | Scala | Python |
| Performance | High (compiled) | Good (interpreted) |
| Startup Time | Slower (JVM warmup) | Fast (native Python) |
| Memory Usage | Higher (JVM overhead) | Lower (Python runtime) |
| Integration | Java/Mule apps | Python apps |
| Module System | Full support | In progress |
| Type System | Static typing | Dynamic typing |

## Roadmap

### Current Status (v0.1.0)
- ✅ Core language parser
- ✅ Expression evaluation
- ✅ 60+ built-in functions
- ✅ Pattern matching
- ✅ Collection operators

### Planned Features
- 🔄 Full module system support
- 🔄 Import statements
- 🔄 Custom function definitions
- 🔄 XML/CSV format support
- 🔄 Streaming for large datasets
- 🔄 Type validation
- 🔄 Performance optimizations

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Write tests for your changes
4. Ensure all tests pass (`pytest`)
5. Commit your changes (`git commit -m 'feat: add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

## License

See the original DataWeave runtime license terms. This project is a reference implementation for educational and development purposes.

## Resources

- [DataWeave Documentation](https://docs.mulesoft.com/dataweave/)
- [DataWeave Tutorial](https://developer.mulesoft.com/tutorials-and-howtos/dataweave/)
- [DataWeave Playground](https://dataweave.mulesoft.com/learn/playground)

## Support

For questions, issues, or contributions:
- Open an issue on GitHub
- Check existing documentation in the `docs/` directory
- Review test cases in `tests/` for usage examples

---

**Note**: This is an independent Python implementation and is not officially supported by MuleSoft. For production use cases requiring full DataWeave compatibility, please use the official JVM-based runtime.

