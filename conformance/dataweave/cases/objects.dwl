// Object literals: dynamic keys, conditional entries, attributes.

// case dynamic-key
// expect {"ab":1}
%dw 2.0
output application/json
---
{("a" ++ "b"): 1}

// case dynamic-key-ident
// expect {"x":1}
%dw 2.0
output application/json
var k='x'
---
{(k): 1}

// case spread-array-of-objects
// expect {"a":1,"b":2}
%dw 2.0
output application/json
---
{([{a:1},{b:2}])}

// case conditional-field
// expect {"a":1,"b":2}
%dw 2.0
output application/json
---
{(a: 1) if true, b: 2}

// case dynamic-attribute
// expect {"a":2}
%dw 2.0
output application/json
---
{a @(("x"): 1): 2}

// case attributes
// expect {"a":3}
%dw 2.0
output application/json
---
{a @(x: 1, y: 2): 3}

// case conditional-attribute
// expect {"a":3}
%dw 2.0
output application/json
---
{a @((x: 1) if true): 3}

// case trailing-comma
// expect {"a":1,"b":2}
%dw 2.0
output application/json
---
{a: 1, b: 2,}

// case keyword-key
// expect error
%dw 2.0
output application/json
---
{if: 1, match: 2, type: 3}

// case dynamic-key-call-builtin
// expect {"A":1}
%dw 2.0
output application/json
---
{(upper('a')): 1}

// case interpolated-key
// expect {"ax":1}
%dw 2.0
output application/json
var k='a'
---
{"$(k)x": 1}

// case dynamic-key-in-array
// expect [{"a":1}]
%dw 2.0
output application/json
---
[{("a"): 1}]

// case spread-mapped
// expect {"k1":1,"k2":2}
%dw 2.0
output application/json
---
{([1,2] map {("k$($)"): $})}

// case spread-object
// expect {"a":1}
%dw 2.0
output application/json
---
{({a:1})}

// case conditional-entries
// expect {"a":1}
%dw 2.0
output application/json
var c = true
---
{(a: 1) if c, (b: 2) if (not c)}

// case conditional-dynamic-key
// expect {"a":1}
%dw 2.0
output application/json
---
{(("a"): 1) if true}

// case conditional-several-entries
// expect error
%dw 2.0
output application/json
---
{(a: 1, b: 2) if true}

// case conditional-attribute-among-others
// expect {"a":3}
%dw 2.0
output application/json
---
{a @((x: 1) if false, y: 2): 3}

// case attributes-spread
// expect {"a":2}
%dw 2.0
output application/json
---
{a @(({x:1})): 2}

// case namespaced-key-and-attribute
// expect {"a":2}
%dw 2.0
output application/json
ns n http://x
---
{n#a @(n#b: 1): 2}

// case quoted-keyword-key
// expect {"if":1}
%dw 2.0
output application/json
---
{"if": 1}

// case unless-entry
// expect error
%dw 2.0
output application/json
---
{(a:1) unless false}

// case number-key
// expect error
%dw 2.0
output application/json
---
{1: 2}

// case dynamic-key-call
// expect {"a":1}
%dw 2.0
output application/json
fun k() = 'a'
---
{(k()): 1}

// case attributes-expression
// expect error
%dw 2.0
output application/json
var at = {x: 1}
---
{a @(at): 2}

// case attributes-spread-variable
// expect {"a":2}
%dw 2.0
output application/json
var at = {x: 1}
---
{a @((at)): 2}

// case attribute-dynamic-key
// expect {"a":2}
%dw 2.0
output application/json
---
{a @(("x"): 1): 2}

// case attribute-conditional-dynamic-key
// expect {"a":2}
%dw 2.0
output application/json
---
{a @((("x"): 1) if true): 2}

// case parenthesized-several-entries
// expect error
%dw 2.0
output application/json
---
{(a: 1, b: 2)}

// case parenthesized-entry
// expect {"a":1}
%dw 2.0
output application/json
---
{(a: 1)}

// case conditional-entry-unparenthesized
// expect error
%dw 2.0
output application/json
---
{a: 1 if true}

// case braceless-conditional
// expect {"a":1}
%dw 2.0
output application/json
---
(a: 1) if true

// case braceless-dynamic-key
// expect {"a":1}
%dw 2.0
output application/json
---
("a"): 1

// case conditional-spread
// expect error
%dw 2.0
output application/json
---
{([{a: 1}, {b: 2}]) if true}

// case conditional-double-parentheses
// expect {"a":1}
%dw 2.0
output application/json
---
{((a: 1)) if true}

// case dynamic-key-attributes
// expect {"a":2}
%dw 2.0
output application/json
---
{("a") @(x: 1): 2}
// case head-tail
// expect {"a":1,"b":2}
%dw 2.0
output application/json
---
{a: 1 ~ {b: 2}}
// case head-tail-variable
// expect {"a":1,"b":2}
%dw 2.0
output application/json
var rest = {b: 2}
---
{a: 1 ~ rest}
// case conditional-parenthesized-then-and
// expect error
%dw 2.0
output application/json
---
{(a: 1) if (true) and (true)}
// case braceless-conditional-parenthesized-then-and
// expect error
%dw 2.0
output application/json
---
(a: 1) if (true) and (true)
