/**
* This utility module calculates the difference between two values
* and returns a list of differences.
*
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::util::Diff` to the header of your
* DataWeave script.
*/
%dw 2.0

// A type: `Diff`.
/**
* Describes the entire difference between two values.
*
* * _Example with no differences:_
* `{ "matches": true, "diffs": [ ] }`
*
* * _Example with differences:_
* `{ "matches": true, "diffs": [ "expected": "4", "actual": "2", "path": "(root).a.@.d" ] }`
*
* See the `diff` function for another example.
*/
type Diff = {
    "matches": Boolean,
    diffs: Array<Difference>
}

// A type: Difference.
/**
* Describes a single difference between two values at a given structure.
*/
type Difference = {
    expected: String,
    actual: String,
    path: String
}

/**
* Returns the structural differences between two values.
*
*
* Differences between objects can be ordered (the default) or unordered. Ordered
* means that two objects do not differ if their key-value pairs are in the same
* order. Differences are expressed as `Difference` type.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | actual | The actual value. Can be any data type.
* | expected | The expected value to compare to the actual. Can be any data type.
* | diffConfig | Setting for changing the default to unordered using `{ "unordered" : true} (explained in the introduction).
* |===
*
* === Example
*
* This example shows a variety of uses of `diff`.
*
* ==== Source
*
* [source,Dataweave,linenums]
* ----
* import diff from dw::util::Diff
* ns ns0 http://locahost.com
* ns ns1 http://acme.com
* output application/dw
* ---
* {
*   "a": diff({a: 1}, {b:1}),
*   "b": diff({ns0#a: 1}, {ns1#a:1}),
*   "c": diff([1,2,3], []),
*   "d": diff([], [1,2,3]),
*   "e": diff([1,2,3], [1,2,3, 4]),
*   "f": diff([{a: 1}], [{a: 2}]),
*   "g": diff({a @(c: 2): 1}, {a @(c: 3): 1}),
*   "h": diff(true, false),
*   "i": diff(1, 2),
*   "j": diff("test", "other test"),
*   "k": diff({a: 1}, {a:1}),
*   "l": diff({ns0#a: 1}, {ns0#a:1}),
*   "m": diff([1,2,3], [1,2,3]),
*   "n": diff([], []),
*   "o": diff([{a: 1}], [{a: 1}]),
*   "p": diff({a @(c: 2): 1}, {a @(c:2): 1}),
*   "q": diff(true, true),
*   "r": diff(1, 1),
*   "s": diff("other test", "other test"),
*   "t": diff({a:1 ,b: 2},{b: 2, a:1}, {unordered: true}),
*   "u": [{format: "ssn",data: "ABC"}] diff [{ format: "ssn",data: "ABC"}]
* }
* ----
*
* ==== Output
*
* [source,XML,linenums]
* ----
* ns ns0 http://locahost.com
* ns ns1 http://acme.com
* ---
* {
*   a: {
*     matches: false,
*     diffs: [
*       {
*         expected: "Entry (root).a with type Number",
*         actual: "was not present in object.",
*         path: "(root).a"
*       }
*     ]
*   },
*   b: {
*     matches: false,
*     diffs: [
*       {
*         expected: "Entry (root).ns0#a with type Number",
*         actual: "was not present in object.",
*         path: "(root).ns0#a"
*       }
*     ]
*   },
*   c: {
*     matches: false,
*     diffs: [
*       {
*         expected: "Array size is 0",
*         actual: "was 3",
*         path: "(root)"
*       }
*     ]
*   },
*   d: {
*     matches: false,
*     diffs: [
*       {
*         expected: "Array size is 3",
*         actual: "was 0",
*         path: "(root)"
*       }
*     ]
*   },
*   e: {
*     matches: false,
*     diffs: [
*       {
*         expected: "Array size is 4",
*         actual: "was 3",
*         path: "(root)"
*       }
*     ]
*   },
*   f: {
*     matches: false,
*     diffs: [
*       {
*         expected: "1" as String {mimeType: "application/dw"},
*         actual: "2" as String {mimeType: "application/dw"},
*         path: "(root)[0].a"
*       }
*     ]
*   },
*   g: {
*     matches: false,
*     diffs: [
*       {
*         expected: "3" as String {mimeType: "application/dw"},
*         actual: "2" as String {mimeType: "application/dw"},
*         path: "(root).a.@.c"
*       }
*     ]
*   },
*   h: {
*     matches: false,
*     diffs: [
*       {
*         expected: "false",
*         actual: "true",
*         path: "(root)"
*       }
*     ]
*   },
*   i: {
*     matches: false,
*     diffs: [
*       {
*         expected: "2",
*         actual: "1",
*         path: "(root)"
*       }
*     ]
*   },
*   j: {
*     matches: false,
*     diffs: [
*       {
*         expected: "\"other test\"",
*         actual: "\"test\"",
*         path: "(root)"
*       }
*     ]
*   },
*   k: {
*     matches: true,
*     diffs: []
*   },
*   l: {
*     matches: true,
*     diffs: []
*   },
*   m: {
*     matches: true,
*     diffs: []
*   },
*   n: {
*     matches: true,
*     diffs: []
*   },
*   o: {
*     matches: true,
*     diffs: []
*   },
*   p: {
*     matches: true,
*     diffs: []
*   },
*   q: {
*     matches: true,
*     diffs: []
*   },
*   r: {
*     matches: true,
*     diffs: []
*   },
*   s: {
*     matches: true,
*     diffs: []
*   },
*   t: {
*     matches: true,
*     diffs: []
*   },
*   u: {
*     matches: true,
*     diffs: []
*   }
* }
* ----
*/
fun diff(actual: Any, expected:Any, diffConfig: {unordered?: Boolean} = {} , path:String = "(root)"): Diff  = do {

    fun createDiff(expected:String, actual:String, path: String): Diff =
        {
            matches: false,
            diffs: [{ expected: expected, actual: actual, path: path }]
        }
    fun createMatch(): Diff =
        {
            matches: true,
            diffs: []
        }

    fun mergeDiff(left:Diff, right:Diff): Diff =
        {
            matches: left.matches and right.matches,
            diffs: left.diffs ++ right.diffs,
        }

    fun keyToString(k:Key):String = do {
        var namespace = if(k.#?) (k.# as Object) else {}
        ---
        if(namespace.prefix?)
            "$(namespace.prefix!)#$(k)"
        else
            k as String
    }

    fun entries(obj: Object): Array<Array<Any>> = obj pluck [$$,$]

    fun namesAreEquals(akey: Key, ekey: Key): Boolean =
        (akey as String == ekey as String) and ((akey.#.uri == ekey.#.uri) or (isEmpty(akey.#.uri) and isEmpty(ekey.#.uri)))

    fun isEmptyAttribute(attrs: Object | Null) =
        attrs == null or attrs == {}

    fun diffAttributes(actual: Any, expected:Any, path:String): Diff = do {
        var actualAttributes = actual.@
        var expectedAttributes = expected.@
        ---
        if(isEmptyAttribute(actualAttributes) and not isEmptyAttribute(expectedAttributes))
            createDiff("Attributes $(write(expectedAttributes))", "Empty attributes.", path)
        else if((not isEmptyAttribute(actualAttributes)) and isEmptyAttribute(expectedAttributes))
            createDiff("Empty attributes", "Attributes $(write(expectedAttributes))", path)
        else if(isEmptyAttribute(expectedAttributes) and isEmptyAttribute(actualAttributes))
            createMatch()
        else
            diff(actualAttributes, expectedAttributes, diffConfig, path)
    }

    fun diffObjects(actual: Object, expected: Object, path: String = "(root)"): Diff = do {
        var aobject = if(diffConfig.unordered default false) actual orderBy $$ else actual
        var eobject = if(diffConfig.unordered default false) expected orderBy $$ else expected
        ---
        if(sizeOf(aobject) == sizeOf(eobject)) do {
            var matchResult = {diff: createMatch(), remaining: aobject }
            ---
            zip(entries(aobject), entries(eobject)) map ((actualExpected) -> do {
                var actualEntry = actualExpected[0]
                var expectedEntry = actualExpected[1]
                var expectedKey = expectedEntry[0]
                var expectedKeyString = keyToString(expectedKey)
                var attributeDiff = diffAttributes(actualEntry[0], expectedKey, "$(path).$(expectedKeyString).@")
                var valueDiff = diff(actualEntry[1], expectedEntry[1], diffConfig, "$(path).$(expectedKeyString)")
                ---
                if(namesAreEquals(actualEntry[0], expectedKey))
                    mergeDiff(attributeDiff, valueDiff)
                else do {
                    var expectedValueType = typeOf(expectedEntry[1])
                    ---
                    createDiff("Entry $(path).$(expectedKeyString) with type $(expectedValueType)", "was not present in object.", "$(path).$(expectedKeyString)")
                }

            })
            reduce ((value, acc = createMatch()) -> mergeDiff(value, acc))
        }
        else
            createDiff("Object size is $(sizeOf(eobject))", "$(sizeOf(aobject))", path)
    }

    ---
    expected match {
        case eobject is Object -> do {
            actual match {
                 case aobject is Object ->
                    diffObjects(aobject, eobject, path)
                 else ->
                    createDiff("Object type", "$(typeOf(actual)) type" , path)
             }
        }
        case earray is Array -> do {
             actual match {
                 case aarray is Array ->
                    if(sizeOf(aarray) == sizeOf(earray))
                      zip(aarray, earray)
                        map ((actualExpected, index) ->
                            diff(actualExpected[0], actualExpected[1], diffConfig, "$(path)[$(index)]")
                        )
                        reduce ((value, acc = createMatch()) ->
                            mergeDiff(value, acc)
                        )
                    else
                      createDiff("Array size is $(sizeOf(earray))", "was $(sizeOf(aarray))" , path)
                 else ->
                    createDiff("Array type", "$(typeOf(actual)) type", path)
             }
        }
        else ->
            if(actual == expected)
              createMatch()
            else
              createDiff("$(write(expected))", "$(write(actual))", path)
    }
}
