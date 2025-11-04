/**
* This module contains helper functions for working with objects.
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::core::Objects` to the header of your
* DataWeave script.
*/
%dw 2.0

/**
 * Returns an array of key-value pairs that describe the key, value, and any
 * attributes in the input object.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | obj | The `Object` to describe.
 * |===
 *
 * === Example
 *
 * This example returns the key, value, and attributes in the object specified
 * in the variable `myVar`.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Objects
 * var myVar = read('<xml attr="x"><a>true</a><b>1</b></xml>', 'application/xml')
 * output application/json
 * ---
 * { "entrySet" : entrySet(myVar) }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * {
 *   "entrySet": [
 *     {
 *        "key": "xml",
 *        "value": {
 *          "a": "true",
 *          "b": "1"
 *        },
 *        "attributes": {
 *          "attr": "x"
 *        }
 *     }
 *   ]
 * }
 * ----
 */
@Deprecated(since = "2.3.0", replacement = "dw::Core::entriesOf")
fun entrySet<T <: Object>(obj: T): Array<{|key: Key, value: Any, attributes: Object|}> = entriesOf(obj)

/**
* Returns an array of keys from an object.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | obj | The object to evaluate.
* |===
*
* === Example
*
* This example returns the keys from the input object.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Objects
* output application/json
* ---
* { "nameSet" : nameSet({ "a" : true, "b" : 1}) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "nameSet" : ["a","b"] }
* ----
*/
@Deprecated(since = "2.3.0", replacement = "dw::Core::namesOf")
fun nameSet(obj: Object): Array<String> = namesOf(obj)

/**
* Returns an array of key names from an object.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | object | The object to evaluate.
* |===
*
* === Example
*
* This example returns the keys from the input object.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Objects
* output application/json
* ---
* { "keySet" : keySet({ "a" : true, "b" : 1}) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "keySet" : ["a","b"] }
* ----
*
* === Example
*
* This example illustrates a difference between `keySet` and `nameSet`.
* Notice that `keySet` retains the attributes (`name` and `lastName`)
* and namespaces (`xmlns`) from the XML input, while `nameSet` returns
* `null` for them because it does not retain them.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Objects
* var myVar = read('<users xmlns="http://test.com">
*                      <user name="Mariano" lastName="Achaval"/>
*                      <user name="Stacey" lastName="Duke"/>
*                   </users>', 'application/xml')
* output application/json
* ---
* { keySetExample: flatten([keySet(myVar.users) map $.#,
*                           keySet(myVar.users) map $.@])
* }
* ++
* { nameSet: flatten([nameSet(myVar.users) map $.#,
*                     nameSet(myVar.users) map $.@])
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "keySet": [
*     "http://test.com",
*     "http://test.com",
*     {
*       "name": "Mariano",
*       "lastName": "Achaval"
*     },
*     {
*       "name": "Stacey",
*       "lastName": "Duke"
*     }
*   ],
*   "nameSet": [
*     null,
*     null,
*     null,
*     null
*   ]
* }
* ----
*/
@Deprecated(since = "2.3.0", replacement = "dw::Core::keysOf")
fun keySet<K,V>(obj: {(K)?: V}): Array<K> = keysOf(obj)

/**
* Returns an array of the values from key-value pairs in an object.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | obj | The object to evaluate.
* |===
*
* === Example
*
* This example returns the values from the input object.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Objects
* output application/json
* ---
* { "valueSet" : valueSet({a: true, b: 1}) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "valueSet" : [true,1] }
* ----
*/
@Deprecated(since = "2.3.0", replacement = "dw::Core::valuesOf")
fun valueSet <K,V>(obj: {(K)?: V}): Array<V> = valuesOf(obj)

/**
 * Appends any key-value pairs from a source object to a target object.
 *
 *
 * If source and target objects have the same key, the function appends
 * that source object to the target and removes that target object from the output.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | source | The object to append to the `target`.
 * | target | The object to which the `source` object is appended.
 * |===
 *
 * === Example
 *
 * This example appends the source objects to the target. Notice that
 * `"a" : true,` is removed from the output, and `"a" : false` is appended
 * to the target.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import mergeWith from dw::core::Objects
 * output application/json
 * ---
 * { "mergeWith" : { "a" : true, "b" : 1} mergeWith { "a" : false, "c" : "Test"} }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * "mergeWith": {
 *     "b": 1,
 *     "a": false,
 *     "c": "Test"
 * }
 * ----
 */
fun mergeWith<T <: Object,V <: Object>(source: T, target: V): ? =
  (source -- keySet(target)) ++ target

/**
* Helper function that enables `mergeWith` to work with a `null` value.
*/
fun mergeWith<T <: Object>(a: Null, b: T): T = b

/**
* Helper function that enables `mergeWith` to work with a `null` value.
*/
fun mergeWith<T <: Object>(a: T, b: Null): T = a

/**
* Breaks up an object into sub-objects that contain the specified number of
* key-value pairs.
*
*
* If there are fewer key-value pairs in an object than the specified number, the
* function will fill the object with those pairs. If there are more pairs, the
* function will fill another object with the extra pairs.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | items | Key-value pairs in the source object.
* | amount | The number of key-value pairs allowed in an object.
* |===
*
* === Example
*
* This example breaks up objects into sub-objects based on the specified `amount`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import divideBy from dw::core::Objects
* output application/json
* ---
* { "divideBy" : {"a": 1, "b" : true, "a" : 2, "b" : false, "c" : 3} divideBy 2 }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "divideBy": [
*     {
*       "a": 1,
*       "b": true
*     },
*     {
*       "a": 2,
*       "b": false
*     },
*     {
*       "c": 3
*     }
*   ]
* }
* ----
*/
fun divideBy(items: Object, amount: Number): Array<{}> = do {
    fun internalDivideBy<T>(items: Object, amount: Number, carry:{} ): Array<{}> =
        items match {
          case {k:v ~ xs} ->
            if(sizeOf(carry) == amount - 1)
                [carry ++ {(k):v} ~ internalDivideBy(xs, amount, {})]
            else
               internalDivideBy(xs, amount, carry ++ {(k):v} )
          else ->
            if(isEmpty(carry))
             []
            else
             [carry]
        }
    ---
    internalDivideBy(items, amount, {})
}

/**
 * Selects key-value pairs from the object while the condition is met.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | obj | The object to filter.
 * | condition | The condition (or expression) used to match a key-value pairs in the object.
 * |===
 *
 * === Example
 *
 * This example iterates over the key-value pairs in the object and selects the elements while the condition is met.
 * It outputs the result into an object.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Objects
 * output application/json
 * var obj = {
 *   "a": 1,
 *   "b": 2,
 *   "c": 5,
 *   "d": 1
 * }
 * ---
 * obj takeWhile ((value, key) ->  value < 3)
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * {
 *   "a": 1,
 *   "b": 2
 * }
 * ----
 */
@Since(version = "2.3.0")
fun takeWhile<T>(obj: Object, condition: (value: Any, key: Key) -> Boolean): Object = do {
  obj match {
    case {} -> obj
    case {k:v ~ tail} ->
      if (condition(v,k))
        {(k): v ~ takeWhile(tail, condition)}
      else
        {}
  }
}

/**
* Returns `true` if every entry in the object matches the condition.
*
*
* The function stops iterating after the first negative evaluation of an
* element in the object.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | object | The object to evaluate.
* | condition | The condition to apply to each element.
* |===
*
* === Example
*
* This example shows how `everyEntry` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import everyEntry from dw::core::Objects
* output application/json
* ---
* {
*     a: {} everyEntry (value, key) -> value is String,
*     b: {a: "", b: "123"} everyEntry (value, key) -> value is String,
*     c: {a: "", b: 123} everyEntry (value, key) -> value is String,
*     d: {a: "", b: 123} everyEntry (value, key) -> key as String == "a",
*     e: {a: ""} everyEntry (value, key) -> key as String == "a",
*     f: null everyEntry ((value, key) -> key as String == "a")
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "a": true,
*   "b": true,
*   "c": false,
*   "d": false,
*   "e": true,
*   "f": true
* }
* ----
**/
@Since(version = "2.3.0")
fun everyEntry(object: Object, condition: (value: Any, key: Key) -> Boolean): Boolean = do {
  object match {
    case {} -> true
      case {k:v ~ tail} ->
        if (condition(v,k))
          everyEntry(tail, condition)
        else
           false
  }
}

/**
* Helper function that enables `everyEntry` to work with a `null` value.
**/
@Since(version = "2.3.0")
fun everyEntry(list: Null, condition: (Nothing, Nothing) -> Boolean): Boolean = true


/**
* Returns `true` if at least one entry in the object matches the specified condition.
*
*
* The function stops iterating after the first element that matches the condition is found.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | obj | The object to evaluate.
* | condition | The condition to use when evaluating elements in the object.
* |===
*
* === Example
*
* This example shows how the `someEntry` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import someEntry from dw::core::Objects
* output application/json
* ---
* {
*     a: {} someEntry (value, key) -> value is String,
*     b: {a: "", b: "123"} someEntry (value, key) -> value is String,
*     c: {a: "", b: 123} someEntry (value, key) -> value is String,
*     d: {a: "", b: 123} someEntry (value, key) -> key as String == "a",
*     e: {a: ""} someEntry (value, key) -> key as String == "b",
*     f: null someEntry (value, key) -> key as String == "a"
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "a": false,
*   "b": true,
*   "c": true,
*   "d": true,
*   "e": false,
*   "f": false
* }
* ----
**/
@Since(version = "2.3.0")
fun someEntry(obj: Object, condition: (value: Any, key: Key) -> Boolean): Boolean = do {
    obj match {
     case {} -> false
     case {k:v ~ tail} ->
        if (condition(v,k))
          true
        else
           someEntry(tail, condition)
    }
}

/**
* Helper function that enables `someEntry` to work with a `null` value.
**/
@Since(version = "2.3.0")
fun someEntry(obj: Null, condition:  (value: Nothing, key: Nothing) -> Boolean): Boolean = false
