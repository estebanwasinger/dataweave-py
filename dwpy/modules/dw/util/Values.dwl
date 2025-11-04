/**
* This utility module simplifies changes to values.
*
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::util::Values` to the header of your
* DataWeave script.
*
*/
@Since(version = "2.2.2")
%dw 2.0

import * from dw::util::Tree

/**
* Type that represents the output type of the update function.
*/
@Since(version = "2.4.0")
type UpdaterValueProvider<ReturnType> = (newValueProvider: (oldValue: Any, index: Number) -> Any) -> ReturnType

/**
* This function creates a `PathElement` to use for selecting an XML
* attribute and populates the type's `selector` field with the given string.
*
*
* Some versions of the `update` and `mask` functions accept a `PathElement` as
* an argument.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | namespace | The namespace of the attribute to select. If not specified, a null value is set.
* | name | The string that names the attribute to select.
* |===
*
* === Example
*
* This example creates an attribute selector for a specified namespace
* (`ns0`) and sets the selector's value to `"myAttr"`. In the
* output, also note that the value of the `"kind"` key is `"Attribute"`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* import * from dw::util::Values
* ns ns0 http://acme.com/fo
* ---
* attr(ns0 , "myAttr")
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "kind": "Attribute",
*    "namespace": "http://acme.com/foo",
*    "selector": "myAttr"
*  }
* ----
**/
@Since(version = "2.2.2")
fun attr(namespace: Namespace | Null = null, name: String): PathElement =
  {
    kind: ATTRIBUTE_TYPE,
    namespace: namespace,
    selector: name
  }

/**
* This function creates a `PathElement` data type to use for selecting an
* _object field_ and populates the type's `selector` field with the given
* string.
*
*
* Some versions of the `update` and `mask` functions accept a `PathElement` as
* an argument.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | namespace | The namespace of the field to select. If not specified, a null value is set.
* | name | A string that names the attribute to select.
* |===
*
* === Example
*
* This example creates an object field selector for a specified namespace
* (`ns0`) and sets the selector's value to `"myFieldName"`. In the
* output, also note that the value of the `"kind"` key is `"Object"`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* import * from dw::util::Values
* ns ns0 http://acme.com/foo
* ---
* field(ns0 , "myFieldName")
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "kind": "Object",
*    "namespace": "http://acme.com/foo",
*    "selector": "myFieldName"
*  }
* ----
**/
@Since(version = "2.2.2")
fun field(namespace: Namespace | Null = null, name: String): PathElement =
  {
    kind: OBJECT_TYPE,
    namespace: namespace,
    selector: name
  }

/**
* This function creates a `PathElement` data type to use for selecting an
* _array element_ and populates the type's `selector` field with the specified
* index.
*
*
* Some versions of the `update` and `mask` functions accept a `PathElement` as
* an argument.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | index | The index.
* |===
*
* === Example
*
* This example creates an selector for a specified index.
* It sets the selector's value to `0`. In the
* output, also note that the value of the `"kind"` key is `"Array"`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* import * from dw::util::Values
* ns ns0 http://acme.com/foo
* ---
* index(0)
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "kind": "Array",
*    "namespace": null,
*    "selector": 0
*  }
* ----
**/
@Since(version = "2.2.2")
fun index(index: Number): PathElement =
  {
    kind: ARRAY_TYPE,
    namespace: null,
    selector: index
  }


/**
* Helper function that enables `mask` to work with a `null` value.
**/
@Since(version = "2.2.2")
fun mask(value: Null, fieldName: String | Number | PathElement): (newValueProvider: (oldValue: Any, path: Path) -> Any) -> Null =
    (newValueProvider: (oldValue: Any, path: Path) -> Any): Null -> null

/**
* This `mask` function replaces all _simple_ elements that match the specified
* criteria.
*
*
* Simple elements do not have child elements and cannot be objects or arrays.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | A value to use for masking. The value can be any DataWeave type.
* | selector | The `PathElement` selector.
* |===
*
* === Example
*
* This example shows how to mask the value of a `password` field in
* an array of objects. It uses `field("password")` to return the `PathElement`
* that it passes to `mask`. It uses `with "*****"` to specify the value
* (`*****`) to use for masking.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* import * from dw::util::Values
* ---
* [{name: "Peter Parker", password: "spiderman"}, {name: "Bruce Wayne", password: "batman"}] mask field("password") with "*****"
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* [
*    {
*      "name": "Peter Parker",
*      "password": "*****"
*    },
*    {
*      "name": "Bruce Wayne",
*      "password": "*****"
*    }
*  ]
* ----
**/
@Since(version = "2.2.2")
@GlobalDescription
fun mask(value: Any, selector: PathElement): (newValueProvider: (oldValue: Any, path: Path) -> Any) -> Any =
  (newValueProvider: (oldValue: Any, path: Path) -> Any): Any -> do {
      mapLeafValues(value, (v, p) -> do {
          var lastSegment = p[-1]
          ---
          if (selector.kind == lastSegment.kind and lastSegment.selector == selector.selector and (selector.namespace == null or lastSegment.namespace == selector.namespace))
            newValueProvider(v, p)
          else
            v
        })
    }

/**
* This `mask` function selects a field by its name.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | The value to use for masking. The value can be any DataWeave type.
* | fieldName | A string that specifies the name of the field to mask.
* |===
*
* === Example
*
* This example shows how to perform masking using the name of a field in the
* input. It modifies the values of all fields with that value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* import * from dw::util::Values
* ---
* [{name: "Peter Parker", password: "spiderman"}, {name: "Bruce Wayne", password: "batman"}] mask "password" with "*****"
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* [
*    {
*      "name": "Peter Parker",
*      "password": "*****"
*    },
*    {
*      "name": "Bruce Wayne",
*      "password": "*****"
*    }
*  ]
* ----
**/
@Since(version = "2.2.2")
fun mask(value: Any, fieldName: String): (newValueProvider: (oldValue: Any, path: Path) -> Any) -> Any =
  mask(value, field(fieldName))

/**
* This `mask` function selects an element from array by its index.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | The value to mask. The value can be any DataWeave type.
* | index | The index to mask. The index must be a number.
* |===
*
* === Example
*
* This example shows how `mask` acts on all elements in the nested arrays.
* It changes the value of each element at index `1` to `false`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* import * from dw::util::Values
* ---
* [[123, true], [456, true]] mask 1 with false
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* [
*    [
*      123,
*      false
*    ],
*    [
*      456,
*      false
*    ]
*  ]
* ----
**/
@Since(version = "2.2.2")
fun mask(value: Any, i: Number): (newValueProvider: (oldValue: Any, path: Path) -> Any) -> Any =
  mask(value, index(i))



/**
* This `update` function updates a field in an object with the specified
* string value.
*
*
* The function returns a new object with the specified field and value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | objectValue | The object to update.
* | fieldName | A string that provides the name of the field.
* |===
*
* === Example
*
* This example updates the `name` field in the object `{name: "Mariano"}` with
* the specified value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Values
* output application/json
* ---
* {name: "Mariano"} update "name" with "Data Weave"
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "name": "Data Weave"
* }
* ----
**/
@Since(version = "2.2.2")
fun update(objectValue: Object, fieldName: String): UpdaterValueProvider<Object> =
  update(objectValue, field(fieldName))

/**
* This `update` function updates an object field with the specified
*  `PathElement` value.
*
*
* The function returns a new object with the specified field and value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | objectValue | The object to update.
* | fieldName | A `PathElement` that specifies the field name.
* |===
*
* === Example
*
* This example updates the value of a `name` field in the object
* `{name: "Mariano"}`. It uses `field("name")` to return the `PathElement`
* that it passes to `update`. It uses `with "Data Weave"` to specify the value
* (`Data Weave`) of `name`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Values
* output application/json
* ---
* {name: "Mariano"} update field("name") with "Data Weave"
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "name": "Data Weave"
* }
* ----
**/
@Since(version = "2.2.2")
fun update(objectValue: Object, fieldName: PathElement): UpdaterValueProvider<Object> =
  (newValueProvider: (oldValue: Any, index: Number) -> Any): Object -> do {
      if (objectValue[fieldName.selector]?)
        objectValue mapObject ((value, key, index) -> if (key ~= fieldName.selector and (fieldName.namespace == null or fieldName.namespace == key.#))
            {
              (key): newValueProvider(value, index)
            }
          else
            {
              (key): value
            })
      else
        objectValue
    }

/**
* Updates an array index with the specified value.
*
*
* This `update` function returns a new array that changes the value of
* the specified index.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | objectValue | The array to update.
* | indexToUpdate | The index of the array to update. The index must be a number.
* |===
*
* === Example
*
* This example replaces the value `2` (the index is `1`) with `5` in the
* the input array `[1,2,3]`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Values
* output application/json
* ---
* [1,2,3] update 1 with 5
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* [
*    1,
*    5,
*    3
*  ]
* ----
**/
@Since(version = "2.2.2")
fun update(arrayValue: Array, indexToUpdate: Number): UpdaterValueProvider<Array> =
  update(arrayValue, index(indexToUpdate))


/**
* This `update` function updates all objects within the specified array with
* the given string value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | objectValue | The array of objects to update.
* | indexToUpdate | A string providing the name of the field to update.
* |===
*
* === Example
*
* This example updates value of the `role` fields in the array of objects.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Values
* output application/json
* ---
* [{role: "a", name: "spiderman"}, {role: "b", name: "batman"}] update "role" with "Super Hero"
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* [{
*    "role": "Super Hero",
*    "name": "spiderman"
*  },
*  {
*    "role": "Super Hero",
*    "name": "batman"
* }]
* ----
**/
@Since(version = "2.2.2")
fun update(arrayValue: Array, indexToUpdate: String): UpdaterValueProvider<Array> =
  update(arrayValue, field(indexToUpdate))

/**
* This `update` function updates the specified index of an array with the
* given `PathElement` value.
*
*
* The function returns a new array that contains given value at
* the specified index.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | objectValue | The array to update.
* | indexToUpdate | The index of the array to update. The index must be specified as a `PathElement`.
* |===
*
* === Example
*
* This example updates the value of an element in the input array. Notice
* that it uses `index(1)` to return the index as a `PathElement`, which
* it passes to `update`. It replaces the value `2` at that index with `5`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Values
* output application/json
* ---
* [1,2,3] update index(1) with 5
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* [
*    1,
*    5,
*    3
*  ]
* ----
**/
@Since(version = "2.2.2")
fun update(arrayValue: Array, indexToUpdate: PathElement): UpdaterValueProvider<Array> =
      (newValueProvider: (oldValue: Any, index: Number) -> Any): Array -> do {
          if(indexToUpdate.kind != ARRAY_TYPE)
                arrayValue map ((item, index) -> item update indexToUpdate with newValueProvider($,$$))
          else
              if (arrayValue[indexToUpdate.selector as Number]?)
                arrayValue map ((value, index) ->
                    if (index == indexToUpdate.selector)
                        newValueProvider(value, index)
                    else
                        value)
              else
                arrayValue
     }

/**
*
* Updates the value at the specified path with the given value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | objectValue | The value to update. Accepts an array, object, or null value.
* | path | The path to update. The path must be an array of strings, numbers, or `PathElement`s.
* |===
*
* === Example
*
* This example updates the name of the user.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Values
* output application/json
* ---
* {user: {name: "Mariano"}} update ["user", field("name")] with "Data Weave"
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "user": {
*      "name": "Data Weave"
*    }
*  }
* ----
*
* === Example
*
* This example updates one of the recurring fields.
*
* ==== Input
*
* [source,XML,linenums]
* ----
* <users>
*   <user>
*     <name>Phoebe</name>
*     <language>French</language>
*   </user>
*   <user>
*     <name>Joey</name>
*     <language>English</language>
*   </user>
* </users>
* ----
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Values
* output application/xml
* ---
* payload update ["users", "user", "language"] with (if ($ == "English") "Gibberish" else $)
* ----
* 
* ==== Output
*
* [source,XML,linenums]
* ----
* <users>
*   <user>
*     <name>Phoebe</name>
*     <language>French</language>
*   </user>
*   <user>
*     <name>Joey</name>
*     <language>Gibberish</language>
*   </user>
* </users>
* ----
*
**/
@Since(version = "2.2.2")
fun update(value: Array | Object | Null, path: Array<String | Number | PathElement>): UpdaterValueProvider< Array | Object | Null> = do {
    fun updateAttr(value: Null, path: Array<String | Number | PathElement>):UpdaterValueProvider<Null> =
      (newValueProvider: (oldValue: Any, index: Number) -> Any) -> null

    fun updateAttr(value: Array | Object | Null, path: Array<String | Number | PathElement>): UpdaterValueProvider<Object | Array<Any> | Null> = do {

      (newValueProvider: (oldValue: Any, index: Number) -> Any) -> do {

          fun doUpdateAttr(objectValue: Object, f: String | Number, attr: String | Number | PathElement): Object = do {
            var fieldName = f as String
            ---
            if (objectValue[fieldName]?)
              objectValue mapObject ((value, key, index) -> if (key ~= fieldName)
                  {
                    (key) @((update(key.@, attr) with newValueProvider($, $$))): value
                  }
                else
                  {
                    (key): value
                  })
            else
              objectValue
          }

          fun doUpdateAttr(objectValue: Object, f: PathElement, attr: String | Number | PathElement): Object = do {
            var fieldName = f.selector as String
            ---
            if (objectValue[fieldName]?)
              objectValue mapObject ((value, key, index) -> if (key ~= fieldName and (f.namespace == null or f.namespace == key.#))
                  {
                    (key) @((update(key.@, attr) with newValueProvider($, $$))): value
                  }
                else
                  {
                    (key): value
                  })
            else
              objectValue
          }

          fun doUpdateAttr(objectValue: Array | Null, fieldName: String | Number | PathElement, attr: String | Number | PathElement): Array<Any> | Null =
            objectValue
          ---
          if (sizeOf(path) > 2)
            value update path[0 to -3] with doUpdateAttr($, path[-2], path[-1])
          else
            doUpdateAttr(value, path[-2], path[-1])
        }
    }
    ---
    if (path[-1] is PathElement and (path[-1] as PathElement).kind == ATTRIBUTE_TYPE)
        updateAttr(value, path)
    else
        (newValueProvider: (oldValue: Any, index: Number) -> Any) -> do {

            fun doRecUpdate(value: Array | Object, path: Array<String | Number | PathElement>): Array<Any> | Object =
              path match {
                case [x ~ xs] -> if (isEmpty(xs))
                  value update x with newValueProvider($, $$)
                else
                  value update x with doRecUpdate($, xs)
                case [] -> value
              }

            fun doRecUpdate(value: Null, path: Array<String | Number | PathElement>): Null =
              value
            ---
            doRecUpdate(value, path)
          }
}

/**
* Helper function that enables `update` to work with a `null` value.
**/
@Since(version = "2.2.2")
fun update(value: Null, toUpdate: Number | String | PathElement): UpdaterValueProvider<Null> =
  (newValueProvider: (oldValue: Any, index: Number) -> Any) -> null
