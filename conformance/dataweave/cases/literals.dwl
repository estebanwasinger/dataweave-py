// Strings, numbers, dates, regexes and array literals.

// case string-interp
// expect "x 1 1"
%dw 2.0
output application/json
var a = 1
---
"x $(a) $a"

// case backtick-str
// expect "abc"
%dw 2.0
output application/json
---
`abc`

// case number-forms
// expect error
%dw 2.0
output application/json
---
[1.5e3, -2, 0x1F]

// case date-forms
// expect ["2020-01-01","12:00:00","P1D","2020-01-01T00:00:00","12:00:00Z","PT1H"]
%dw 2.0
output application/json
---
[|2020-01-01|, |12:00:00|, |P1D|, |2020-01-01T00:00:00|, |12:00:00Z|, |PT1H|]

// case period-arithmetic
// expect "2020-01-02"
%dw 2.0
output application/json
---
|2020-01-01| + |P1D|

// case unicode-esc
// expect "é"
%dw 2.0
output application/json
---
"\u00e9"

// case string-escapes
// expect "\t\n\\\"$"
%dw 2.0
output application/json
---
"\t\n\\\"\$"

// case single-quote-interp
// expect "x 1"
%dw 2.0
output application/json
var a = 1
---
'x $(a)'

// case backtick-interp
// expect "x 1"
%dw 2.0
output application/json
var a = 1
---
`x $(a)`

// case nested-interp
// expect "1 in 1"
%dw 2.0
output application/json
var a = {b: 1}
---
"$(a.b) $("in $(a.b)")"

// case dollar-ident-interp
// expect "1."
%dw 2.0
output application/json
var a = 1
---
"$a."

// case regex-flags
// expect error
%dw 2.0
output application/json
---
'A' matches /a/i

// case regex-escape
// expect true
%dw 2.0
output application/json
---
'a/b' matches /a\/b/

// case number-exp
// expect [1E+3,0.015,0.5]
%dw 2.0
output application/json
---
[1e3, 1.5E-2, 0.5]

// case number-leading-dot
// expect error
%dw 2.0
output application/json
---
.5

// case date-offset
// expect "2020-01-01T00:00:00+01:00"
%dw 2.0
output application/json
---
|2020-01-01T00:00:00+01:00|

// case date-tz
// expect error
%dw 2.0
output application/json
---
|2020-01-01T00:00:00[America/New_York]|

// case date-local
// expect "2020-01-01T00:00:00.123"
%dw 2.0
output application/json
---
|2020-01-01T00:00:00.123|

// case time-zone-literal
// expect "-03:00"
%dw 2.0
output application/json
---
|-03:00|

// case period-combo
// expect error
%dw 2.0
output application/json
---
|P1Y2M3DT4H5M6S|

// case range-literal
// expect [1,2,3]
%dw 2.0
output application/json
---
1 to 3

// case trailing-comma-array
// expect [1,2]
%dw 2.0
output application/json
---
[1,2,]

// case trailing-comma-args
// expect error
%dw 2.0
output application/json
---
upper('a',)

// case array-conditional-item
// expect [1,2]
%dw 2.0
output application/json
---
[(1) if true, 2]

// case array-conditional-items
// expect [2]
%dw 2.0
output application/json
---
[(1) if false, (2) if true]

// case multiline-string
// expect "a\nb"
%dw 2.0
output application/json
---
"a
b"

// case array-unless-item
// expect error
%dw 2.0
output application/json
---
[(1) unless false]

// case array-conditional-item-unparenthesized
// expect error
%dw 2.0
output application/json
---
[1 if true]
// case array-head-tail
// expect [1,2,3]
%dw 2.0
output application/json
---
[1 ~ [2, 3]]
// case array-head-tail-variable
// expect [1,2,3]
%dw 2.0
output application/json
var xs = [2, 3]
---
[1 ~ xs]
// case array-head-tail-chained
// expect error
%dw 2.0
output application/json
---
[1 ~ 2 ~ [3]]
// case array-head-tail-unbracketed
// expect error
%dw 2.0
output application/json
---
1 ~ [2]
// case array-head-tail-after-items
// expect error
%dw 2.0
output application/json
---
[0, 1 ~ [2]]
// case array-conditional-parenthesized-then-and
// expect error
%dw 2.0
output application/json
---
[(1) if (true) and (true)]
// case array-conditional-and
// expect [1]
%dw 2.0
output application/json
---
[(1) if true and true]
