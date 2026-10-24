// match: literal, type, regex, guard and deconstruction patterns.

// case object-key-value-rest
// expect "a"
%dw 2.0
output application/json
---
{a:1,b:2} match { case {k: v ~ rest} -> k else -> 0 }

// case array-head-tail-recursion
// expect 6
%dw 2.0
output application/json
fun f(xs) = xs match { case [] -> 0 case [h ~ t] -> h + f(t) }
---
f([1,2,3])

// case array-head-tail
// expect 1
%dw 2.0
output application/json
---
[1,2] match { case [x ~ xs] -> x else -> 0 }

// case object-empty
// expect 1
%dw 2.0
output application/json
---
{} match { case {} -> 1 else -> 2 }

// case literal-guard
// expect "one"
%dw 2.0
output application/json
---
1 match { case 1 -> 'one' case x if x > 1 -> 'big' else -> 'no' }

// case is
// expect 1
%dw 2.0
output application/json
---
1 match { case is Number -> 1 case s is String -> 2 }

// case regex
// expect "b"
%dw 2.0
output application/json
---
'ab' match { case m matches /a(b)/ -> m[1] else -> '' }

// case ident-literal
// expect 1
%dw 2.0
output application/json
---
1 match { case x: 1 -> x else -> 0 }

// case regex-literal-pattern
// expect error
%dw 2.0
output application/json
---
'ab' match { case /a.*/ -> 1 else -> 0 }

// case array-literal-pattern
// expect error
%dw 2.0
output application/json
---
[1] match { case [1] -> 1 else -> 0 }

// case object-literal-pattern
// expect error
%dw 2.0
output application/json
---
{a:1} match { case {a: 1} -> 1 else -> 0 }

// case array-nested-pattern
// expect error
%dw 2.0
output application/json
---
[[1,2]] match { case [[a ~ b] ~ rest] -> a else -> 0 }

// case inside-update
// expect {"a":2}
%dw 2.0
output application/json
---
{a:1} update { case v at .a -> v match { case 1 -> 2 else -> 3 } }

// case string-match-number-literal
// expect 1
%dw 2.0
output application/json
---
'a' match { case 'a' -> 1 case "b" -> 2 else -> 3 }

// case type-pattern-with-guard
// expect error
%dw 2.0
output application/json
---
1 match { case n is Number if n > 0 -> 1 else -> 0 }

// case array-empty
// expect 1
%dw 2.0
output application/json
---
[] match { case [] -> 1 else -> 2 }

// case array-empty-named
// expect error
%dw 2.0
output application/json
---
[] match { case x: [] -> 1 else -> 2 }

// case array-head-tail-named
// expect error
%dw 2.0
output application/json
---
[1] match { case a: [h ~ t] -> a else -> 2 }

// case array-head-tail-guard
// expect error
%dw 2.0
output application/json
---
[1] match { case [h ~ t] if h > 0 -> h else -> 2 }

// case array-single-item
// expect error
%dw 2.0
output application/json
---
[1] match { case [h] -> h else -> 2 }

// case array-head-typed
// expect error
%dw 2.0
output application/json
---
[1] match { case [h: Number ~ t] -> h else -> 2 }

// case array-literal-head
// expect error
%dw 2.0
output application/json
---
[1] match { case [1 ~ t] -> t else -> 2 }

// case array-two-heads
// expect error
%dw 2.0
output application/json
---
[1,2,3] match { case [a ~ b ~ c] -> c else -> 2 }

// case object-empty-named
// expect error
%dw 2.0
output application/json
---
{} match { case o: {} -> 1 else -> 2 }

// case object-rest-guard
// expect error
%dw 2.0
output application/json
---
{a:1} match { case {k: v ~ r} if v > 0 -> r else -> 2 }

// case object-rest-named
// expect error
%dw 2.0
output application/json
---
{a:1} match { case o: {k: v ~ r} -> o else -> 2 }

// case object-key-value-rest-names
// expect 1
%dw 2.0
output application/json
---
{a:1} match { case {a: v ~ r} -> v else -> 2 }

// case object-quoted-key-name
// expect error
%dw 2.0
output application/json
---
{a:1} match { case {"a": v ~ r} -> v else -> 2 }

// case no-case-matches
// expect error
%dw 2.0
output application/json
---
1 match { case 2 -> 1 }

// case neg-literal
// expect 1
%dw 2.0
output application/json
---
-1 match { case -1 -> 1 else -> 2 }

// case date-literal-pat
// expect 1
%dw 2.0
output application/json
---
|2020-01-01| match { case |2020-01-01| -> 1 else -> 2 }

// case named-literal
// expect "a"
%dw 2.0
output application/json
---
'a' match { case s: 'a' -> s else -> 2 }

// case is-generic
// expect 1
%dw 2.0
output application/json
---
[1] match { case is Array<Number> -> 1 else -> 2 }

// case is-union
// expect 1
%dw 2.0
output application/json
---
1 match { case is Number | String -> 1 else -> 2 }

// case is-literal-type
// expect 1
%dw 2.0
output application/json
---
'a' match { case is 'a' -> 1 else -> 2 }

// case regex-unnamed
// expect 2
%dw 2.0
output application/json
---
'ab' match { case matches /a/ -> 1 else -> 2 }

// case guard-parenthesized
// expect 1
%dw 2.0
output application/json
---
1 match { case x if (x > 0) -> 1 else -> 2 }

// case else-before-case
// expect 2
%dw 2.0
output application/json
---
1 match { else -> 2 case 1 -> 1 }

// case match-regex-operand
// expect ["ab","a","b"]
%dw 2.0
output application/json
---
'ab' match /(a)(b)/

// case match-as-field-value
// expect {"a":2}
%dw 2.0
output application/json
---
{a: 1 match { case 1 -> 2 else -> 3 }}

// case select-from-match
// expect 1
%dw 2.0
output application/json
---
(1 match { else -> {a:1} }).a

// case guard-parenthesized-then-and
// expect error
%dw 2.0
output application/json
---
1 match { case x if (x > 0) and (x < 2) -> 2 else -> 3 }

// case cases-with-commas
// expect error
%dw 2.0
output application/json
---
1 match { case 1 -> 2, else -> 3 }
