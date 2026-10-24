// Operators and their precedence.

// case metadata-assign
// expect "x"
%dw 2.0
output application/json
---
'x' <~ {class: 'Foo'}

// case negate-parenthesized
// expect -1
%dw 2.0
output application/json
---
-(1)

// case concat-strings
// expect "ab"
%dw 2.0
output application/json
---
'a' ++ 'b'

// case date-difference
// expect "PT24H"
%dw 2.0
output application/json
---
|2020-01-02| - |2020-01-01|

// case compare-strings
// expect true
%dw 2.0
output application/json
---
'a' < 'b'

// case concat-objects
// expect {"a":1,"b":2}
%dw 2.0
output application/json
---
{a: 1} ++ {b: 2}

// case is-in-condition
// expect 1
%dw 2.0
output application/json
---
if (1 is Number) 1 else 2

// case not
// expect false
%dw 2.0
output application/json
---
not true

// case prepend-append
// expect [1,2,3]
%dw 2.0
output application/json
---
(1 >> [2]) << 3

// case concat
// expect [1,2]
%dw 2.0
output application/json
---
[1] ++ [2]

// case not-and-or
// expect false
%dw 2.0
output application/json
---
not true and (true or false)

// case bang-not
// expect false
%dw 2.0
output application/json
---
!true

// case similar
// expect true
%dw 2.0
output application/json
---
1 ~= '1'

// case math-ops
// expect [3,2,6,3,1]
%dw 2.0
output application/json
---
[1 + 2, 3 - 1, 2 * 3, 6 / 2, 7 mod 2]

// case precedence
// expect 5
%dw 2.0
output application/json
---
1 + 2 * 3 - 4 / 2

// case as-chain
// expect 1
%dw 2.0
output application/json
---
1 as String as Number

// case default-chain
// expect 1
%dw 2.0
output application/json
---
null default null default 1

// case on-expr
// expect false
%dw 2.0
output application/json
---
!(1 == 1)

// case and-or-precedence
// expect true
%dw 2.0
output application/json
---
true or false and false

// case unary-minus-var
// expect -1
%dw 2.0
output application/json
var a = 1
---
-a

// case prepend-append-unparenthesized
// expect [1,2,3]
%dw 2.0
output application/json
---
1 >> [2] << 3

// case triple-equals
// expect error
%dw 2.0
output application/json
---
1 === 1

// case is
// expect true
%dw 2.0
output application/json
---
1 is Number
