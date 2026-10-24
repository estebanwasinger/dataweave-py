// Syntax forms found missing by comparing with the engine's own test suite; scripts written for this corpus.

// case literal-number-union-type
// expect 2
%dw 2.0
output application/json
type Small = 1 | 2 | 3
var s: Small = 2
---
s
// case ordinal-date
// expect "2005-02-14"
%dw 2.0
output application/json
---
|2005-045|
// case week-date
// expect "2005-01-03"
%dw 2.0
output application/json
---
|2005-W01-1|
// case period-days-and-time
// expect "PT36H"
%dw 2.0
output application/json
---
|P1DT12H|
// case period-weeks
// expect error
%dw 2.0
output application/json
---
|P1W|
// case period-date-and-time
// expect error
%dw 2.0
output application/json
---
|P1Y2M3DT4H|
// case date-with-space
// expect error
%dw 2.0
output application/json
---
|2020-01-01 |
// case annotated-value
// expect {"a":1}
%dw 2.0
output application/json
@Metadata(key = "label")
annotation Label(value: String)
---
@Label(value = "x") {a: 1}
// case annotated-type
// expect "a"
%dw 2.0
output application/json
@Metadata(key = "label")
annotation Label(value: String)
type T = @Label(value = "x") String
---
"a" as T
// case annotated-type-key
// expect {"a":1}
%dw 2.0
output application/json
@Metadata(key = "key")
annotation Key(value: Number)
type T = { @Key(value = 1) a: Number }
---
{a: 1} as T
// case annotated-object-key
// expect {"a":1}
%dw 2.0
output application/json
@Metadata(key = "label")
annotation Label(value: String)
---
{@Label(value = "x") a: 1}
// case annotated-if
// expect 1
%dw 2.0
output application/json
@Metadata(key = "label")
annotation Label(value: String)
fun f(a) = @Label(value = "x") if (a) 1 else 2
---
f(true)
// case metadata-selector-then-directive
// expect null
%dw 2.0
fun f(a) = a.^
output application/json
---
f(1).class
// case namespaced-type-selector
// expect 1
%dw 2.0
output application/json
ns n http://x
type T = {n#a: Number}
type U = T.n#a
---
1 as U
// case namespaced-attribute-type
// expect 1
%dw 2.0
output application/json
ns n http://x
type T = {n#a @(n#b: String): Object}
---
1
// case conditional-head-tail
// expect [2,3]
%dw 2.0
output application/json
---
[(1) if (false) ~ [2, 3]]
// case conditional-head-tail-included
// expect [1,2,3]
%dw 2.0
output application/json
---
[(1) if (true) ~ [2, 3]]
// case dynamic-key-head-tail
// expect {"a":1,"b":2}
%dw 2.0
output application/json
---
{("a"): 1 ~ {b: 2}}
// case descendants-attribute
// expect ["1"]
%dw 2.0
output application/json
---
(read("<a><b id=\"1\"/></a>", "application/xml"))..@id
// case bracket-multi-attribute
// expect ["1","2"]
%dw 2.0
output application/json
---
(read("<a><b id=\"1\"/><b id=\"2\"/></a>", "application/xml")).a.*b[*@id]
// case named-else
// expect "x"
%dw 2.0
output application/json
---
"x" match { case "a" -> 1 else v -> v }
// case update-value-and-index
// expect {"a":1,"a":3}
%dw 2.0
output application/json
---
{a: 1, a: 2} update { case (v, i) at .*a -> v + i }
// case negative-literal-type-argument
// expect {"r":-1}
%dw 2.0
output application/json
type R<T> = {r: T}
var x: R<-1> = {r: -1}
---
x
// case java-static-field
// expect error
%dw 2.0
output application/json
---
java!java::lang::Integer::MAX_VALUE
// case interpolator-call
// expect {"q":"SELECT ? FROM t","v":[1]}
%dw 2.0
output application/json
fun SQL(parts: Array<String>, values: Array<Any>) = {q: parts joinBy "?", v: values}
---
SQL `SELECT $(1) FROM t`
// case values-update-with
// expect {"a":{"b":2}}
%dw 2.0
output application/json
import update from dw::util::Values
---
{a: {b: 1}} update ["a", "b"] with 2
// case values-mask-with
// expect {"a":"*","b":2}
%dw 2.0
output application/json
import mask from dw::util::Values
---
{a: 1, b: 2} mask "a" with "*"
// case annotation-after-plain-output
// expect error
%dw 2.0
output application/json
annotation Label(value: String)
---
1
// case annotation-before-output
// expect 1
%dw 2.0
annotation Label(v: String)
output application/json
---
1
// case annotation-after-output-options
// expect 1
%dw 2.0
output application/json indent=false
annotation Label(v: String)
---
1

// case namespace-selector
// expect {"r":"http://a.com"}
%dw 2.0
output application/json
ns n http://a.com
---
{r: {n#a @(x: 1): 2}.a.#.uri}

// case period-fields
// expect {"h":2,"d":2}
%dw 2.0
output application/json
---
{h: (|2010-12-10T12:10:12| - |2010-12-10T10:02:10|).hours, d: |P1Y2M3D|.months}

// case filter-index
// expect [20,30]
%dw 2.0
output application/json
---
[10, 20, 30] filter ($$ > 0)

// case lambda-dollars
// expect [0,1]
%dw 2.0
output application/json
---
[1, 2] map ((x, i) -> i) map $$

// case using-sequential
// expect 2
%dw 2.0
output application/json
---
using (a = 1, b = a + 1) b

// case output-type
// expect 1
%dw 2.0
output application/json
type T = Number
---
1 as T

// case try-runtime
// expect false
%dw 2.0
import try from dw::Runtime
output application/json
---
try(() -> 1 / 0).success

// case temporal-arithmetic
// expect {"a":"2020-01-02","b":2,"c":"2020-01-02T10:00:00Z"}
%dw 2.0
output application/json
---
{a: |2020-01-01| + |P1D|, b: (|2020-01-03| - |2020-01-01|).days, c: |P1D| + |2020-01-01T10:00:00Z|}
