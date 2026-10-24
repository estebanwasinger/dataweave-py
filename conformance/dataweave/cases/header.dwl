// Script headers: directives, imports, annotations.

// case namespace
// expect {"a":1}
%dw 2.0
output application/json
ns ns0 http://x
---
{ns0#a: 1}

// case import-module
// expect "A"
%dw 2.0
output application/json
import dw::core::Strings
---
Strings::capitalize('a')

// case import-ns-call
// expect "A"
%dw 2.0
output application/json
---
dw::core::Strings::capitalize('a')

// case annotation
// expect 0
%dw 2.0
output application/json
@TailRec()
fun f(n) = if (n <= 0) 0 else f(n - 1)
---
f(3)

// case input-directive
// expect 1
%dw 2.0
output application/json
input payload application/json
---
1

// case output-props
// expect 1
%dw 2.0
output application/json indent=false, skipNullOn='everywhere'
---
1

// case output-with-ns
// expect text "<?xml version='1.0' encoding='UTF-8'?>\n<a>1</a>"
%dw 2.0
output application/xml
---
{a: 1}

// case var-with-type
// expect 1
%dw 2.0
output application/json
var a: Number = 1
---
a

// case comments
// expect 1
%dw 2.0
output application/json
// x
/* y */
---
1 // z

// case import-star
// expect 1
%dw 2.0
output application/json
import * from dw::core::Arrays
---
countBy([1], (x) -> true)

// case import-alias-module
// expect error
%dw 2.0
output application/json
import dw::core::Strings as S
---
S::upper('a')

// case import-in-body
// expect 1
%dw 2.0
output application/json
---
do { import dw::core::Strings --- 1 }

// case output-csv-properties
// expect text "1\n"
%dw 2.0
output application/csv separator=';', header=false, lineSeparator='\n'
---
[{a:1}]

// case input-named
// expect 1
%dw 2.0
output application/json
input in0 application/json
---
1

// case no-version
// expect 1
%dw 2.0
output application/json
---
1

// case import-star-objects
// expect ["a"]
%dw 2.0
output application/json
import * from dw::core::Objects
---
keySet({a:1})

// case private-var
// expect 1
%dw 2.0
output application/json
private var a = 1
---
a

// case annotation-lazy
// expect 1
%dw 2.0
output application/json
@Lazy
var v = 1
---
v

// case annotation-interceptor
// expect 1
%dw 2.0
output application/json
@Interceptor(interceptorFunction = "#foo")
annotation A()
---
1

// case comment-nested
// expect error
%dw 2.0
output application/json
/* a /* b */ c */
---
1

// case import-from-wrong-module-used
// expect error
%dw 2.0
output application/json
import upper as u from dw::core::Strings
---
u('a')

// case import-unknown
// expect error
%dw 2.0
output application/json
import nope from dw::core::Strings
---
1

// case private-fun
// expect 1
%dw 2.0
output application/json
private fun f() = 1
---
f()
// case import-alias
// expect "A"
%dw 2.0
output application/json
import capitalize as cap from dw::core::Strings
---
cap('a')
// case import-several
// expect "aBC"
%dw 2.0
output application/json
import capitalize, camelize from dw::core::Strings
---
camelize('a_b') ++ capitalize('c')
