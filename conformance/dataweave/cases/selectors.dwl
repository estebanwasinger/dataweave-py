// Selectors.

// case key-value-pair
// expect {"a":1}
%dw 2.0
output application/json
---
{a:1}.&a

// case all-attributes
// expect {"x":1}
%dw 2.0
output application/json
---
{a @(x:1): 1}.a.@

// case attribute
// expect 1
%dw 2.0
output application/json
---
{a @(x:1): 1}.a.@x

// case multi-value-attribute
// expect [1]
%dw 2.0
output application/json
---
{a @(x:1): 1}.a.*@x

// case quoted-field
// expect 1
%dw 2.0
output application/json
---
{"a-b": 1}."a-b"

// case dynamic-index
// expect 1
%dw 2.0
output application/json
var k='a'
---
{a:1}[k]

// case keyword-field
// expect error
%dw 2.0
output application/json
---
{type: 1}.type

// case key-value-pair-dynamic
// expect error
%dw 2.0
output application/json
var k='a'
---
{a:1}.&(k)

// case field-of-literal
// expect 1
%dw 2.0
output application/json
---
{a:1}.a

// case namespace-selector
// expect 1
%dw 2.0
output application/json
ns ns0 http://x
---
{ns0#a: 1}.ns0#a

// case descendants
// expect [1]
%dw 2.0
output application/json
---
{a:{b:1}}..b

// case index-selector
// expect 1
%dw 2.0
output application/json
---
{a:1, b:2}["a"]

// case range-selector
// expect [1,2]
%dw 2.0
output application/json
---
[1,2,3][0 to 1]

// case neg-index
// expect 3
%dw 2.0
output application/json
---
[1,2,3][-1]

// case string-range
// expect "ab"
%dw 2.0
output application/json
---
'abc'[0 to 1]

// case filter-selector
// expect [{"a":1}]
%dw 2.0
output application/json
---
[{a:1}][?($.a == 1)]

// case assertion-selector
// expect 1
%dw 2.0
output application/json
---
{a:1}.a!

// case null-safe-question
// expect true
%dw 2.0
output application/json
---
{a:1}.a?

// case namespace-of-object
// expect null
%dw 2.0
output application/json
---
{a:1}.#

// case metadata-class
// expect null
%dw 2.0
output application/json
---
{a:1}.^class

// case nullsafe-chained
// expect null
%dw 2.0
output application/json
---
null.a.b

// case dyn-paren
// expect error
%dw 2.0
output application/json
var k='a'
---
{a:1}.(k)

// case quoted-interp
// expect 1
%dw 2.0
output application/json
var k='a'
---
{a:1}."$(k)"

// case multi-dyn
// expect error
%dw 2.0
output application/json
var k='a'
---
{a:1}.*(k)

// case descendants-star
// expect [1]
%dw 2.0
output application/json
---
{a:{b:1}}..*b

// case all-values
// expect error
%dw 2.0
output application/json
---
{a:1, b:2}.*

// case star-bare
// expect error
%dw 2.0
output application/json
---
{a:1}[*]

// case index-star
// expect 1
%dw 2.0
output application/json
---
[[1]][0][0]

// case filter
// expect [2]
%dw 2.0
output application/json
---
[1,2][?($ > 1)]

// case attributes-of-element
// expect {"x":1}
%dw 2.0
output application/json
---
{a @(x:1): 1}.a.@

// case ns-star
// expect [1]
%dw 2.0
output application/json
ns n http://x
---
{n#a:1}.*n#a

// case metadata-all
// expect {}
%dw 2.0
output application/json
---
'a'.^

// case mimetype
// expect null
%dw 2.0
output application/json
---
'a'.^mimeType

// case namespace-of-value
// expect null
%dw 2.0
output application/json
---
{a:1}.a.#

// case exists
// expect false
%dw 2.0
output application/json
---
{a:1}.b?

// case descendants-nested
// expect [{"a":1},1]
%dw 2.0
output application/json
---
{a:{a:1}}..a

// case chained-index-range
// expect "ello"
%dw 2.0
output application/json
---
'hello'[1 to -1]
