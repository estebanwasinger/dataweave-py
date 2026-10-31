// The update operator: selectors, guards, upserts.

// case basic
// expect {"a":2}
%dw 2.0
output application/json
---
{a:1} update { case .a -> 2 }

// case cond
// expect {"a":2}
%dw 2.0
output application/json
---
{a:1} update { case x at .a if (x > 0) -> x + 1 }

// case cond-noparen
// expect error
%dw 2.0
output application/json
---
{a:1} update { case x at .a if x > 0 -> x + 1 }

// case upsert
// expect {"a":1,"b":2}
%dw 2.0
output application/json
---
{a:1} update { case .b! -> 2 }

// case multi
// expect error
%dw 2.0
output application/json
---
{a:[{b:1}]} update { case .a[*].b -> 2 }

// case index
// expect [9,2]
%dw 2.0
output application/json
---
[1,2] update { case [0] -> 9 }

// case attr
// expect {"a":1}
%dw 2.0
output application/json
---
{a @(x:1): 1} update { case .a.@x -> 2 }

// case dynamic
// expect {"a":2}
%dw 2.0
output application/json
var k='a'
---
{a:1} update { case ."$(k)" -> 2 }

// case multivalue
// expect {"a":3,"a":3}
%dw 2.0
output application/json
---
{a:1, a:2} update { case .*a -> 3 }

// case chained-guard
// expect [{"a":2}]
%dw 2.0
output application/json
---
[{a:1}] update { case [0].a if ($ == 1) -> 2 }

// case dyn-paren
// expect error
%dw 2.0
output application/json
var k='a'
---
{a:1} update { case .($(k)) -> 2 }

// case dyn-paren-ident
// expect error
%dw 2.0
output application/json
var k='a'
---
{a:1} update { case .(k) -> 2 }

// case attr-dyn
// expect error
%dw 2.0
output application/json
var k='x'
---
{a @(x:1): 1} update { case .a.@(k) -> 2 }

// case nested
// expect {"a":{"b":{"c":2}}}
%dw 2.0
output application/json
---
{a:{b:{c:1}}} update { case .a.b.c -> 2 }

// case root
// expect error
%dw 2.0
output application/json
---
{a:1} update { case $ -> 2 }

// case index-neg
// expect [1,9]
%dw 2.0
output application/json
---
[1,2] update { case [-1] -> 9 }

// case dynamic-index
// expect [9,2]
%dw 2.0
output application/json
var i = 0
---
[1,2] update { case [i] -> 9 }

// case upsert-nested
// expect {"a":{"b":1}}
%dw 2.0
output application/json
---
{} update { case .a.b! -> 1 }

// case upsert-idx
// expect [1]
%dw 2.0
output application/json
---
[] update { case [0]! -> 1 }

// case guard-uses
// expect {"a":2}
%dw 2.0
output application/json
---
{a:1} update { case .a if ($ > 0) -> $ + 1 }

// case several
// expect {"a":0,"b":4}
%dw 2.0
output application/json
---
{a:1, b:2} update { case .a -> 0 case x at .b -> x * 2 }

// case comma
// expect error
%dw 2.0
output application/json
---
{a:1, b:2} update { case .a -> 0, case .b -> 1 }

// case quoted-key
// expect {"a b":0}
%dw 2.0
output application/json
---
{"a b":1} update { case ."a b" -> 0 }

// case ns
// expect {"a":0}
%dw 2.0
output application/json
ns n http://x
---
{n#a:1} update { case .n#a -> 0 }

// case multi-value
// expect {"a":0}
%dw 2.0
output application/json
---
{a:1} update { case .*a -> 0 }

// case desc
// expect error
%dw 2.0
output application/json
---
{a:{b:1}} update { case ..b -> 0 }

// case at-with-paren-guard-name
// expect {"a":1}
%dw 2.0
output application/json
---
{a:1} update { case v at .a if (v == 1) -> v }

// case in-pipeline
// expect [{"a":2}]
%dw 2.0
output application/json
---
[{a:1}] map ($ update { case .a -> 2 })

// case guard-parenthesized-then-and
// expect error
%dw 2.0
output application/json
---
{a: 1} update { case x at .a if (x > 0) and (x < 2) -> 2 }

// case index-string-literal
// expect error
%dw 2.0
output application/json
---
{a: 1} update { case ["a"] -> 2 }

// case attribute-nested
// expect {"a":1}
%dw 2.0
output application/json
---
{a @(x: 1): 1} update { case .a.@x -> 2 }

// case multi-value-attribute
// expect error
%dw 2.0
output application/json
---
{a @(x: 1): 1} update { case .a.*@x -> 2 }

// case interpolated-key
// expect {"a":2}
%dw 2.0
output application/json
var k = "a"
---
{a: 1} update { case ."$(k)" -> 2 }

// case index-with-string
// expect error
%dw 2.0
output application/json
var k = "a"
---
{a: 1} update { case [k] -> 2 }

// case attribute-bracket
// expect error
%dw 2.0
output application/json
---
{a @(x: 1): 1} update { case .a.@['x'] -> 2 }

// case attribute-bracket-parenthesized
// expect error
%dw 2.0
output application/json
---
{a @(x: 1): 1} update { case .a.@[( 'x' )] -> 2 }

// case attribute-quoted
// expect {"a":1}
%dw 2.0
output application/json
---
{a @(x: 1): 1} update { case .a.@'x' -> 2 }

// case dynamic-key-parenthesized
// expect error
%dw 2.0
output application/json
var k = 'a'
---
{a: 1} update { case .(k) -> 2 }
