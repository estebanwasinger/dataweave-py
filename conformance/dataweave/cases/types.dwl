// Type expressions and declarations.

// case closed-object
// expect 1
%dw 2.0
output application/json
type A = {| a: String |}
---
1

// case ordered-object
// expect 1
%dw 2.0
output application/json
type A = {- a: String -}
---
1

// case key-type
// expect 1
%dw 2.0
output application/json
type T = Key
---
1

// case optional-key
// expect 1
%dw 2.0
output application/json
type O = {a?: String}
---
1

// case key-attributes
// expect 1
%dw 2.0
output application/json
type O = {a @(x: String): String}
---
1

// case namespaced-key
// expect 1
%dw 2.0
output application/json
ns n http://x
type O = {n#a: String}
---
1

// case wildcard-key
// expect 1
%dw 2.0
output application/json
type O = {_: String}
---
1

// case repeated-key
// expect 1
%dw 2.0
output application/json
type O = {a*: String}
---
1

// case builtin-type-names
// expect 1
%dw 2.0
output application/json
type T = Key | Namespace | Uri | Regex | Range | Period | LocalDateTime | TimeZone | Binary | SimpleType | Comparable | Dictionary<String> | Iterator | Type<String> | CData | Any | Nothing
---
1

// case regex-key
// expect error
%dw 2.0
output application/json
type O = {(/a.*/): String}
---
1

// case array-of-object
// expect 1
%dw 2.0
output application/json
type T = Array<{a: String}>
---
1

// case field-type-access
// expect 1
%dw 2.0
output application/json
type T = {a: String}
type U = T.a
---
1

// case function-type
// expect "x"
%dw 2.0
output application/json
fun f(g: (Number) -> String): String = g(1)
---
f((n) -> 'x')

// case params
// expect 1
%dw 2.0
output application/json
fun id<T>(x: T): T = x
---
id(1)

// case def
// expect {"name":"a"}
%dw 2.0
output application/json
type Person = {name: String, age?: Number}
---
{name:'a'} as Person

// case union-literal
// expect "a"
%dw 2.0
output application/json
type C = 'a' | 'b'
---
'a' as C

// case intersection
// expect 1
%dw 2.0
output application/json
type A = {a: String} & {b: Number}
---
1

// case fun-overloads
// expect 2
%dw 2.0
output application/json
fun f(a: String) = 1
fun f(a: Number) = 2
---
f(1)

// case with-props
// expect 1
%dw 2.0
output application/json
type D = Date {format: 'yyyy'}
---
1

// case ref-of
// expect 1
%dw 2.0
output application/json
type T = {a: String}
type U = T.a
---
1

// case of-ordered
// expect 1
%dw 2.0
output application/json
type O = {- a: String, b: Number -}
---
1

// case closed-ordered
// expect 1
%dw 2.0
output application/json
type O = {-| a: String |-}
---
1

// case function
// expect 1
%dw 2.0
output application/json
type F = (a: String, b?: Number) -> String
---
1

// case generic
// expect 1
%dw 2.0
output application/json
type B<T> = {v: T}
---
1

// case bounded-generic
// expect "a"
%dw 2.0
output application/json
fun f<T <: String>(x: T): T = x
---
f('a')

// case metadata
// expect 1
%dw 2.0
output application/json
type T = String {format: 'x'}
---
1

// case metadata-alt
// expect 1
%dw 2.0
output application/json
type T = String <~ {format: 'x'}
---
1

// case literal-number
// expect 1
%dw 2.0
output application/json
type T = 1 | 2
---
1

// case literal-bool
// expect 1
%dw 2.0
output application/json
type T = true
---
1

// case array-short
// expect 1
%dw 2.0
output application/json
type T = Array<String>
---
1

// case nested-union-paren
// expect 1
%dw 2.0
output application/json
type T = (String | Number) & String
---
1

// case of-fn
// expect "Number"
%dw 2.0
output application/json
---
typeOf(1)

// case fun-overload-typed
// expect null
%dw 2.0
output application/json
fun f(a: String): String = a
fun f(a: Null): Null = a
---
f(null)
// case intersection-objects-merge
// expect {"a":"x","b":1}
%dw 2.0
output application/json
type A = {a: String} & {b: Number}
fun f(x: A): A = x
---
f({a: "x", b: 1})
// case intersection-conflicting-fields
// expect {"a":"x"}
%dw 2.0
output application/json
type A = {a: String} & {a: Number}
var x: A = {a: "x"}
---
x
// case intersection-primitives
// expect error
%dw 2.0
output application/json
var x: String & Number = "a"
---
x
// case generic-applied
// expect {"items":["a"],"total":1}
%dw 2.0
output application/json
type Page<T> = {items: Array<T>, total: Number}
fun f(p: Page<String>): Page<String> = p
---
f({items: ["a"], total: 1})
// case generic-nested
// expect {"l":{"v":1},"r":["x"]}
%dw 2.0
output application/json
type Box<T> = {v: T}
type Pair2<L, R> = {l: Box<L>, r: Array<R>}
fun f(p: Pair2<Number, String>): Pair2<Number, String> = p
---
f({l: {v: 1}, r: ["x"]})
// case generic-intersection
// expect {"id":"a","meta":1}
%dw 2.0
output application/json
type Base = {id: String}
type WithMeta<T> = T & {meta: Number}
fun f(p: WithMeta<Base>): WithMeta<Base> = p
---
f({id: "a", meta: 1})
// case generic-without-arguments
// expect {"v":1}
%dw 2.0
output application/json
type Box<T> = {v: T}
fun f(p: Box): Box = p
---
f({v: 1})
// case generic-wrong-argument-count
// expect error
%dw 2.0
output application/json
type Box<T> = {v: T}
fun f(p: Box<String, Number>) = p
---
f({v: 1})
// case generic-argument-mismatch
// expect error
%dw 2.0
output application/json
type Box<T> = {v: T}
fun f(p: Box<String>) = p
---
f({v: [1]})
// case intersection-field-num
// expect {"a":1}
%dw 2.0
output application/json
type A = {a: String} & {a: Number}
var x: A = {a: 1}
---
x
// case intersection-field-bool
// expect error
%dw 2.0
output application/json
type A = {a: String} & {a: Number}
var x: A = {a: true}
---
x
// case intersection-field-fun-num
// expect {"a":1}
%dw 2.0
output application/json
type A = {a: String} & {a: Number}
fun f(x: A) = x
---
f({a: 1})
// case intersection-field-fun-str
// expect {"a":"s"}
%dw 2.0
output application/json
type A = {a: String} & {a: Number}
fun f(x: A) = x
---
f({a: "s"})
// case intersection-field-fun-bool
// expect error
%dw 2.0
output application/json
type A = {a: String} & {a: Number}
fun f(x: A) = x
---
f({a: true})
// case intersection-field-fun-missing
// expect error
%dw 2.0
output application/json
type A = {a: String} & {b: Number}
fun f(x: A) = x
---
f({a: "s"})
// case intersection-field-closed
// expect {"a":"s","b":1}
%dw 2.0
output application/json
type A = {|a: String|} & {b: Number}
fun f(x: A) = x
---
f({a: "s", b: 1})
// case intersection-literal
// expect error
%dw 2.0
output application/json
type A = String & "x"
fun f(x: A) = x
---
f("y")
// case metadata-property-if
// expect error
%dw 2.0
output application/json
---
"2020" as Date {format: "yyyy" if true}
// case metadata-property-if-declared
// expect error
%dw 2.0
output application/json
type D = Date {format: "yyyy" if true}
---
"2020" as D
// case metadata-parenthesized-entry
// expect error
%dw 2.0
output application/json
---
"2020" as Date {format: "yyyy", (x: 1)}
// case metadata-unknown-reference
// expect error
%dw 2.0
output application/json
---
"2020" as Date {format: payload}
// case metadata-variable
// expect error
%dw 2.0
output application/json
var f = "yyyy"
---
"2020" as Date {format: f}
// case metadata-quoted-key
// expect error
%dw 2.0
output application/json
---
"2020" as Date {"format": "yyyy"}
// case metadata-expression
// expect error
%dw 2.0
output application/json
---
"2020" as Date {format: "yy" ++ "yy"}
// case metadata-date-if-unparenthesized
// expect error
%dw 2.0
output application/json
---
"2020" as Date {(format: "yyyy") if true}
// case metadata-string-if-unparenthesized
// expect error
%dw 2.0
output application/json
---
"a" as String {(format: "x") if true}
// case metadata-declared-if-unparenthesized
// expect error
%dw 2.0
output application/json
type T = String {(format: "x") if true}
---
"a" as T
// case metadata-number-if-unparenthesized
// expect error
%dw 2.0
output application/json
---
1 as String {(format: "#.0") if true}
// case metadata-number-if-false-unparenthesized
// expect error
%dw 2.0
output application/json
---
1 as String {(format: "#.0") if false}
// case metadata-conditional-property
// expect "1.0"
%dw 2.0
output application/json
---
1 as String {(format: "#.0") if (true), locale: "en"}
// case metadata-conditional-property-variable-unparenthesized
// expect error
%dw 2.0
output application/json
var c = true
---
1 as String {(format: "#.0") if c, class: "x"}
// case metadata-declared-date-if-unparenthesized
// expect error
%dw 2.0
output application/json
type T = Date {(format: "yyyy") if true}
---
"2020" as T
// case metadata-var-cond
// expect "1.0"
%dw 2.0
output application/json
var c = true
---
1 as String {(format: "#.0") if (c), class: "x", locale: "en"}
// case metadata-declared
// expect error
%dw 2.0
output application/json
type T = Date {(format: "yyyy") if (true)}
---
"2020" as T
// case metadata-false-cond
// expect "1"
%dw 2.0
output application/json
---
1 as String {(format: "#.0") if (false)}
// case metadata-cond-expr
// expect "1.0"
%dw 2.0
output application/json
---
1 as String {(format: "#.0") if (1 > 0), locale: "en"}
// case metadata-cond-paren-and
// expect error
%dw 2.0
output application/json
---
1 as String {(format: "#.0") if (true) and (true)}
// case metadata-unparenthesized-entry
// expect error
%dw 2.0
output application/json
---
1 as String {format: "#.0" if (true)}
// case metadata-two-entries
// expect error
%dw 2.0
output application/json
---
1 as String {(format: "#.0", class: "x") if (true)}
// case metadata-dynamic-key
// expect error
%dw 2.0
output application/json
---
1 as String {("format"): "#.0"}
// case metadata-dynamic-key-cond
// expect error
%dw 2.0
output application/json
---
1 as String {(("format"): "#.0") if (true)}
// case metadata-spread
// expect error
%dw 2.0
output application/json
---
1 as String {({format: "#.0"})}
// case metadata-unless
// expect error
%dw 2.0
output application/json
---
1 as String {(format: "#.0") unless (false)}
// case recursive-mutual
// expect {"child":[{"child":[{"child":null}]}]}
%dw 2.0
output application/json
type A = {child: B}
type B = Array<A> | Null
fun f(a: A): A = a
---
f({child: [{child: [{child: null}]}]})
// case recursive-json
// expect {"a":[1,{"b":["x"]}]}
%dw 2.0
output application/json
type J = String | Number | Array<J> | {_?: J}
fun f(j: J): J = j
---
f({a: [1, {b: ["x"]}]})
// case recursive-mismatch
// expect error
%dw 2.0
output application/json
type Tree = {v: Number, kids: Array<Tree>}
fun f(t: Tree) = t
---
f({v: 1, kids: [{v: "x", kids: []}]})
// case recursive-tree
// expect {"v":1,"kids":[{"v":2,"kids":[{"v":3,"kids":[{"v":4,"kids":[{"v":5,"kids":[]}]}]}]}]}
%dw 2.0
output application/json
type Tree = {v: Number, kids: Array<Tree>}
fun f(t: Tree): Tree = t
---
f({v: 1, kids: [{v: 2, kids: [{v: 3, kids: [{v: 4, kids: [{v: 5, kids: []}]}]}]}]})
