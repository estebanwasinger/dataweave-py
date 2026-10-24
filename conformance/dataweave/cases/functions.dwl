// Functions, lambdas, do blocks and if/unless.

// case do-var-expression
// expect 2
%dw 2.0
output application/json
---
do { var x = 1 --- x + 1 }

// case using
// expect 2
%dw 2.0
output application/json
---
using (x = 1) x + 1

// case unless-otherwise
// expect error
%dw 2.0
output application/json
---
unless (true) 1 otherwise 2

// case lambda-shorthand
// expect [1,3]
%dw 2.0
output application/json
---
[1,2] map $$ + $

// case infix-call
// expect 3
%dw 2.0
output application/json
fun add(a, b) = a + b
---
1 add 2

// case default-params
// expect 3
%dw 2.0
output application/json
fun f(a, b = 2) = a + b
---
f(1)

// case lambda-in-var
// expect 2
%dw 2.0
output application/json
var f = (a) -> a + 1
---
f(1)

// case lambda-typed
// expect 2
%dw 2.0
output application/json
var f = (a: Number): Number -> a + 1
---
f(1)

// case pipe-chain
// expect 5
%dw 2.0
output application/json
---
[1,2] map $ + 1 filter $ > 1 reduce $ + $$

// case reduce-acc-init
// expect 3
%dw 2.0
output application/json
---
[1,2] reduce (item, acc = 0) -> acc + item

// case mapobject
// expect {"a":2}
%dw 2.0
output application/json
---
{a:1} mapObject (v, k) -> {(k): v + 1}

// case pluck
// expect ["a"]
%dw 2.0
output application/json
---
{a:1} pluck $$

// case cdata
// expect text "<?xml version='1.0' encoding='UTF-8'?>\n<a><![CDATA[x]]></a>"
%dw 2.0
output application/xml
---
{a: 'x' as CData}

// case as-with-props
// expect "2020-01-01"
%dw 2.0
output application/json
---
'2020-01-01' as Date {format: 'yyyy-MM-dd'}

// case if-elseif
// expect 2
%dw 2.0
output application/json
---
if (false) 1 else if (true) 2 else 3

// case parenthesized-lambda-call
// expect 3
%dw 2.0
output application/json
---
((x) -> x + 1)(2)

// case nested-fun
// expect 1
%dw 2.0
output application/json
fun f() = do { fun g() = 1 --- g() }
---
f()

// case distinctby
// expect [1,2]
%dw 2.0
output application/json
---
[1,2] distinctBy $

// case lambda-default
// expect 2
%dw 2.0
output application/json
var f = (a, b = 1) -> a + b
---
f(1)

// case lambda-no-params
// expect 1
%dw 2.0
output application/json
var f = () -> 1
---
f()

// case lambda-return-type
// expect [1]
%dw 2.0
output application/json
---
[1] map ((x): Number -> x)

// case then
// expect 2
%dw 2.0
output application/json
---
1 then $ + 1

// case dollar3-mapobject
// expect {"a":0}
%dw 2.0
output application/json
---
{a:1} mapObject {($$): $$$}

// case dollar3-pluck
// expect [0]
%dw 2.0
output application/json
---
{a:1} pluck $$$

// case do-with-type
// expect "a"
%dw 2.0
output application/json
---
do { type T = String --- 'a' as T }

// case do-with-fun
// expect 1
%dw 2.0
output application/json
---
do { fun f() = 1 --- f() }

// case do-with-ns
// expect {"a":1}
%dw 2.0
output application/json
---
do { ns n http://x --- {n#a: 1} }

// case if-no-paren
// expect error
%dw 2.0
output application/json
---
if true 1 else 2

// case if-as-value-in-obj
// expect {"a":1}
%dw 2.0
output application/json
---
{a: if (true) 1 else 2}

// case infix-lambdas-chained
// expect ["1"]
%dw 2.0
output application/json
---
[1] map (x) -> x reduce (a, b) -> a + b

// case implicit-lambda-dynamic-key
// expect [{"1":1}]
%dw 2.0
output application/json
---
[1] map {($ as String): $}

// case mapobject-dynamic-key
// expect {"A":1}
%dw 2.0
output application/json
---
{a:1} mapObject {(upper($$)): $}

// case lambda-index-pair
// expect [1]
%dw 2.0
output application/json
---
[[1,2]] map (pair) -> pair[0]

// case filterobject-key
// expect {"a":1}
%dw 2.0
output application/json
---
{a:1} filterObject $$ ~= 'a'

// case implicit-lambda-object
// expect [{"v":1}]
%dw 2.0
output application/json
---
[1] map {v: $}

// case if-without-else
// expect error
%dw 2.0
output application/json
---
if (true) 1

// case unless-else
// expect 2
%dw 2.0
output application/json
---
unless (true) 1 else 2

// case unknown-infix-function
// expect error
%dw 2.0
output application/json
---
1 otherwise 2

// case do-namespace
// expect {"a":1}
%dw 2.0
output application/json
---
do { ns n http://x --- {n#a: 1} }

// case do-var
// expect {"a":1}
%dw 2.0
output application/json
---
do { var x = 1 --- {a: x} }
// case undefined-body
// expect 1
%dw 2.0
output application/json
fun f() = ???
---
1
// case undefined-called
// expect error
%dw 2.0
output application/json
fun f() = ???
---
f()
