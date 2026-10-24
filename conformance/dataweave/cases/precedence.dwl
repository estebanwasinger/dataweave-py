// Operator precedence, what may follow match/update/is/do, unary operators; found by fuzzing.

// case 1-2-a
// expect null
%dw 2.0
output application/json
---
[1, 2].a
// case 1-block-a
// expect [2]
%dw 2.0
output application/json
---
[1, {a: 2}].a
// case block-a
// expect null
%dw 2.0
output application/json
---
[{b: 1}].a
// case a
// expect null
%dw 2.0
output application/json
---
[].a
// case block-a-2
// expect null
%dw 2.0
output application/json
---
[[{a: 1}]].a
// case block
// expect {"a":1}
%dw 2.0
output application/json
---
{(a: 1) if [1] contains 1}
// case a-1-if-match-block
// expect {"a":1}
%dw 2.0
output application/json
---
{(a: 1) if "" match { else -> true }}
// case block-2
// expect {"a":1}
%dw 2.0
output application/json
---
{(a: 1) if null default true}
// case 1-if-1-contains-1
// expect [1]
%dw 2.0
output application/json
---
[(1) if [1] contains 1]
// case block-update-block-a
// expect error
%dw 2.0
output application/json
---
{a: 1} update { case .a -> 2 } - "a"
// case block-update-block-block
// expect {"a":2,"b":1}
%dw 2.0
output application/json
---
{a: 1} update { case .a -> 2 } ++ {b: 1}
// case block-update-block-a-2
// expect 2
%dw 2.0
output application/json
---
({a: 1} update { case .a -> 2 }).a
// case block-update-block-default-1
// expect {"a":2}
%dw 2.0
output application/json
---
{a: 1} update { case .a -> 2 } default 1
// case 1-match-block-1
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } - 1
// case a-match-block-c
// expect "bc"
%dw 2.0
output application/json
---
"a" match { else -> "b" } ++ "c"
// case 1-match-block-map
// expect [2]
%dw 2.0
output application/json
---
[1] match { else -> [2] } map $
// case 1-map-x-x-0
// expect error
%dw 2.0
output application/json
---
[1] map ((x) -> x)[0]
// case 1-match-block-1-2
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } == 1
// case 1-match-block-1-2-2
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } != 1
// case 1-match-block-1-2-2-2
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } > 1
// case 1-match-block-and-true
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } and true
// case 1-match-block-or-true
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } or true
// case 1-match-block-as-string
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } as String
// case 1-match-block-is-number
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } is Number
// case 1-match-block-1-2-2-2-2
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } + 1
// case 1-match-block-2
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } * 2
// case 1-match-block-default-1
// expect 2
%dw 2.0
output application/json
---
1 match { else -> 2 } default 1
// case 1-match-block-1-2-2-2-2-2
// expect "21"
%dw 2.0
output application/json
---
1 match { else -> 2 } ++ 1
// case 1-match-block-1-2-2-2-2-2-2
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } >> [1]
// case 1-match-block-1-2-2-2-2-2-2-2
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } << 1
// case 1-match-block-1-2-2-2-2-2-2-2-2
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } ~= 1
// case 1-match-block-to-3
// expect [2,3]
%dw 2.0
output application/json
---
1 match { else -> 2 } to 3
// case 1-match-block-match-block
// expect 1
%dw 2.0
output application/json
---
1 match { else -> 2 } match { else -> 1 }
// case 1-match-block-update-block
// expect 2
%dw 2.0
output application/json
---
1 match { else -> 2 } update { case .a -> 1 }
// case 1-match-block-a
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } .a
// case 1-match-block-0
// expect error
%dw 2.0
output application/json
---
1 match { else -> 2 } [0]
// case 1-is-number-as-string
// expect error
%dw 2.0
output application/json
---
1 is Number as String
// case 1-is-number-true
// expect true
%dw 2.0
output application/json
---
1 is Number == true
// case 1-is-number-and-true
// expect true
%dw 2.0
output application/json
---
1 is Number and true
// case 1-is-number-or-false
// expect true
%dw 2.0
output application/json
---
1 is Number or false
// case 1-is-number-default-1
// expect true
%dw 2.0
output application/json
---
1 is Number default 1
// case 1-is-number-x
// expect "truex"
%dw 2.0
output application/json
---
1 is Number ++ "x"
// case 1-is-number
// expect [true]
%dw 2.0
output application/json
---
[1 is Number]
// case 1-is-string-number
// expect true
%dw 2.0
output application/json
---
1 is String | Number
// case 1-is-number-as-string-2
// expect "true"
%dw 2.0
output application/json
---
(1 is Number) as String
// case do-var-a-block-a-b
// expect error
%dw 2.0
output application/json
---
do { var a = {b: 1} --- a }.b
// case using-u-block-u-b
// expect 1
%dw 2.0
output application/json
---
using (u = {b: 1}) u.b
// case if-true-block-else-block-b
// expect {"b":1}
%dw 2.0
output application/json
---
if (true) {b: 1} else {b: 2}.b
// case 1-as-string-is-string
// expect true
%dw 2.0
output application/json
---
1 as String is String
// case 1-as-number-as-string
// expect "1"
%dw 2.0
output application/json
---
1 as Number as String
// case 1-as-string-x
// expect "1x"
%dw 2.0
output application/json
---
1 as String ++ "x"
// case 1-as-string-1
// expect true
%dw 2.0
output application/json
---
1 as String == "1"
// case if-true-true-else-false
// expect error
%dw 2.0
output application/json
---
!if (true) true else false
// case if-true-true-else-false-2
// expect false
%dw 2.0
output application/json
---
!(if (true) true else false)
// case not-if-true-true-else-false
// expect false
%dw 2.0
output application/json
---
not if (true) true else false
// case if-true-1-else-2
// expect -1
%dw 2.0
output application/json
---
-if (true) 1 else 2
// case not-true
// expect error
%dw 2.0
output application/json
---
!not true
// case not-not-true
// expect true
%dw 2.0
output application/json
---
not not true
// case true
// expect error
%dw 2.0
output application/json
---
!!true
// case not-true-2
// expect true
%dw 2.0
output application/json
---
not !true
// case 1
// expect 1
%dw 2.0
output application/json
---
-(-1)
// case 1-2
// expect error
%dw 2.0
output application/json
---
- -1
// case if-true-1-else-2-match-block-1
// expect 2
%dw 2.0
output application/json
---
if (true) 1 else 2 match { else -> 3 } + 1
// case if-true-1-else-2-match-block
// expect 1
%dw 2.0
output application/json
---
if (true) 1 else 2 match { else -> 3 }
// case 0-match-case-1-1-match-block-1-else-0
// expect error
%dw 2.0
output application/json
---
0 match { case 1 -> 1 match { else -> 2 } + 1 else -> 0 }
// case if-true-1-else-2-match-block-1-2
// expect [2]
%dw 2.0
output application/json
---
[if (true) 1 else 2 match { else -> 3 } + 1]
// case if-false-1-else-2-match-block
// expect 3
%dw 2.0
output application/json
---
if (false) 1 else 2 match { else -> 3 }
// case 1-as-string
// expect "-1"
%dw 2.0
output application/json
---
-1 as String
// case 1-as-string-2
// expect -1
%dw 2.0
output application/json
---
-(1) as String
// case true-as-string
// expect "false"
%dw 2.0
output application/json
---
!true as String
// case not-true-as-string
// expect false
%dw 2.0
output application/json
---
not true as String
// case 2-as-number
// expect -2
%dw 2.0
output application/json
---
-"2" as Number
// case 1-is-number-2
// expect true
%dw 2.0
output application/json
---
-1 is Number
// case false-is-boolean
// expect true
%dw 2.0
output application/json
---
!false is Boolean
// case 1-0
// expect -1
%dw 2.0
output application/json
---
-[1][0]
// case block-a-2-2
// expect -1
%dw 2.0
output application/json
---
-{a: 1}.a
// case block-2-2
// expect {"a":1,"a":2}
%dw 2.0
output application/json
---
{a: 1, a: 2}
// case block-a-2-2-2
// expect 1
%dw 2.0
output application/json
---
{a: 1, a: 2}.a
// case block-a-2-2-2-2
// expect [1,2]
%dw 2.0
output application/json
---
{a: 1, a: 2}.*a
// case not-before-do
// expect error
%dw 2.0
output application/json
---
!do { var t = true --- t }
// case not-before-using
// expect error
%dw 2.0
output application/json
---
!using (u = true) u
// case not-before-unless
// expect error
%dw 2.0
output application/json
---
!unless (true) true else false
// case is-as-in-else-branch
// expect "false"
%dw 2.0
output application/json
---
if (false) null else null is Array as String
// case is-as-in-default-operand
// expect error
%dw 2.0
output application/json
---
-3 default 0 is Number as String
// case braceless-dynamic-entry-then-is
// expect error
%dw 2.0
output application/json
---
{(("1"): [] match { else -> 1 } is Object) if true}
// case update-absent-target-body-not-run
// expect {}
%dw 2.0
output application/json
---
{} update { case .a.b -> 1 to "a" as Number }
// case equality-kinds-strict
// expect [false,true,false,true,false,true]
%dw 2.0
output application/json
---
[1 == "1", 1 == 1.0, true == "true", null == null, null == 1, "a" != "b"]
// case equality-structures
// expect [true,true,false]
%dw 2.0
output application/json
---
[[1] == [1], {a: 1} == {a: 1}, |2020-01-01| == "2020-01-01"]
// case if-literal-condition-type
// expect true
%dw 2.0
output application/json
---
if ("a" == 2) "a" as Number > 0 else true
// case lambda-body-block-then-is
// expect []
%dw 2.0
output application/json
---
[] map ((x1) -> [] match { else -> 1 } is Object)
// case array-select-empty
// expect null
%dw 2.0
output application/json
---
[].a
// case object-repeated-keys
// expect {"a":null,"a":1}
%dw 2.0
output application/json
---
{a: null ~ {a: 1}}
// case negate-converted
// expect 1.5
%dw 2.0
output application/json
---
-(-(1.5)) as String {format: "#.0", locale: "en"}
// case selector-after-space
// expect 1
%dw 2.0
output application/json
---
{a: 1} .a
// case index-after-space
// expect 1
%dw 2.0
output application/json
---
[1, 2] [0]
// case match-block-then-selector
// expect error
%dw 2.0
output application/json
---
1 match { else -> {a: 2} }.a
// case update-block-then-selector
// expect error
%dw 2.0
output application/json
---
{a: 1} update { case .a -> 2 }.a
// case match-block-then-index
// expect error
%dw 2.0
output application/json
---
1 match { else -> [2] }[0]
// case lambda-body-block-then-selector
// expect error
%dw 2.0
output application/json
---
[1, 2] map ((x1) -> 0 match { else -> {} }.a?)
// case descendants-none
// expect null
%dw 2.0
output application/json
---
{}..a
// case descendants-missing
// expect null
%dw 2.0
output application/json
---
{a: {a: 1}}..zz
// case remove-keys-list
// expect {"b":2}
%dw 2.0
output application/json
---
{a: 1, b: 2} -- ["a"]
// case remove-keys-all
// expect {}
%dw 2.0
output application/json
---
{a: 1} -- ["a"]
// case remove-key-missing
// expect {"a":1}
%dw 2.0
output application/json
---
{a: 1} - "b"
// case key-is-not-string
// expect [false]
%dw 2.0
output application/json
---
{a: 1} pluck $$ is String
// case key-is-key
// expect [true]
%dw 2.0
output application/json
---
{a: 1} pluck $$ is Key
// case key-coerces-to-string
// expect ["ax"]
%dw 2.0
output application/json
---
{a: 1} pluck ($$ ++ "x")
// case key-upper
// expect {"A":1}
%dw 2.0
output application/json
---
{a: 1} mapObject {(upper($$)): $}
// case default-after-map
// expect [null]
%dw 2.0
output application/json
---
[null] map $ default 7
// case default-after-concat
// expect error
%dw 2.0
output application/json
---
"x" ++ null default "y"
// case concat-after-default
// expect "1a"
%dw 2.0
output application/json
---
null default 1 ++ "a"
// case default-after-mod
// expect 0
%dw 2.0
output application/json
---
"" as Number mod max([].a) default 0
// case default-after-failed-coercion
// expect 3
%dw 2.0
output application/json
---
1 + "a" as Number default 3
// case default-chain
// expect 1
%dw 2.0
output application/json
---
null default null default 1
// case mod-by-zero-default
// expect error
%dw 2.0
output application/json
---
7 mod 0 default 5
// case contains-default
// expect true
%dw 2.0
output application/json
---
[1, 2] contains 1 default false
// case key-exists-null-value
// expect true
%dw 2.0
output application/json
---
{a: null}.a?
// case key-missing
// expect false
%dw 2.0
output application/json
---
{}.a?
// case key-on-null
// expect false
%dw 2.0
output application/json
---
null.a?
// case key-on-array
// expect true
%dw 2.0
output application/json
---
[{a: 1}].a?
// case dead-branch-from-existence
// expect 0
%dw 2.0
output application/json
---
if ({}.a?) "a" as Number else 0
// case open-rest-repeated-key
// expect {"a":0,"a":1}
%dw 2.0
output application/json
---
{a: 0 ~ {a: 1} mapObject ((v1, k1) -> {(k1): v1})}
// case update-repeated-keys
// expect {"a":null,"a":1}
%dw 2.0
output application/json
---
{a: null ~ {a: 1}} update { case .a.b -> null }
// case default-after-match-else
// expect "10:00:00"
%dw 2.0
output application/json
---
1 match { case is String -> |2020-01-01| else -> |10:00:00| } default |10:00:00| + |P1D|
// case matches-literal-true
// expect true
%dw 2.0
output application/json
---
"ab" matches /a.*/
// case matches-literal-false
// expect false
%dw 2.0
output application/json
---
"b c" matches /a.*/
// case matches-whole-text
// expect false
%dw 2.0
output application/json
---
"ab" matches /a/
// case replace-without-match
// expect "b c"
%dw 2.0
output application/json
---
"b c" replace /a/ with "b" as Number
// case replace-with-match
// expect "b c"
%dw 2.0
output application/json
---
"a c" replace /a/ with "b"
// case case-matches-non-string
// expect 0
%dw 2.0
output application/json
---
true match { case s matches /a(.*)/ -> "a" as Number else -> 0 }
// case case-matches-literal
// expect ["ab","a","b"]
%dw 2.0
output application/json
---
"ab" match { case m matches /(a)(b)/ -> m else -> null }
// case union-keeps-repeated-keys
// expect {"a":{},"a":1}
%dw 2.0
output application/json
---
if ("a" <= 'x') {a: {} ~ {a: 1}} else {}
// case union-with-open-object
// expect [{"a":1},{"a":{}},null]
%dw 2.0
output application/json
---
flatten([valuesOf({a: 1} update { case .a -> {a: 1} }), [({a: 1} mapObject ((v1, k1) -> {(k1): {}})) if true, null]])
// case dead-entry-literal-matches
// expect {}
%dw 2.0
output application/json
---
{(([] contains {} as String): "a") if "b c" matches /a.*/}
// case var-evaluated-eagerly
// expect error
%dw 2.0
output application/json
var v = "a" as Number
---
1
// case var-evaluated-eagerly-used-nowhere-else
// expect 1
%dw 2.0
output application/json
var v = upper(1)
---
1
// case conditional-entry-then-selector
// expect error
%dw 2.0
output application/json
---
[({}) if do { var t = "1" --- t }[0 to 0] matches /a.*/, {}]
// case is-as-ends-else-through-default
// expect "a"
%dw 2.0
output application/json
---
if (true) "a" else true default [] is Null as String
// case callback-lambda-compared
// expect {}
%dw 2.0
output application/json
---
{} filterObject ((v) -> v != null) == "a"
// case update-first-repeated-key
// expect {"a":0,"a":1}
%dw 2.0
output application/json
---
{a: 1} ++ {a: 1} update { case .a -> 0 }
// case dynamic-entry-update-selector
// expect error
%dw 2.0
output application/json
---
{(("a"): {} update { case .b! -> "a" }.*a) if true}
// case is-as-in-array
// expect error
%dw 2.0
output application/json
---
[1 is Number as String]
