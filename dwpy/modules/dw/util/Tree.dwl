/**
* This utility module provides functions for handling values as tree-data structures.
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::util::Tree` to the header of your
* DataWeave script.
*/
@Since(version = "2.2.2")
%dw 2.0

/**
* Variable used to identify a `PathElement` value as an object.
*/
@Since(version = "2.2.2")
var OBJECT_TYPE = "Object"

/**
* Variable used to identify a `PathElement` value as an attribute.
*/
@Since(version = "2.2.2")
var ATTRIBUTE_TYPE = "Attribute"

/**
* Variable used to identify a `PathElement` value as an array.
*/
@Since(version = "2.2.2")
var ARRAY_TYPE = "Array"

/**
* Type that consists of an array of `PathElement` values that
* identify the location of a node in a tree. An example is
* `[{kind: OBJECT_TYPE, selector: "user", namespace: null}, {kind: ATTRIBUTE_TYPE, selector: "name", namespace: null}] as Path`.
*/
@Since(version = "2.2.2")
type Path = Array<PathElement>

/**
* Type that represents a selection of a node in a path.
* An example is `{kind: ARRAY_TYPE, selector: "name", namespace: null} as PathElement`.
*/
@Since(version = "2.2.2")
type PathElement = {|
        kind: "Object" | "Attribute" | "Array" ,
        selector: String | Number,
        namespace: Namespace | Null
    |}


/**
* Transforms a `Path` value into a string representation of the path.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | path | The `Path` value to transform into a `String` value.
* |===
*
* === Example
*
* This example transforms a `Path` value into a `String` representation
* of a selector for an attribute of an object.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Tree
* output application/json
* ---
* asExpressionString([
*         {kind: OBJECT_TYPE, selector: "user", namespace: null},
*         {kind: ATTRIBUTE_TYPE, selector: "name", namespace: null}
*     ])
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* ".user.@name"
* ----
**/
@Since(version = "2.2.2")
fun asExpressionString(path: Path): String =
    path reduce ((item, accumulator = "") -> do {
        var selectorExp = item.kind match {
            case "Attribute" -> ".@$(item.selector as String)"
            case "Array" -> "[$(item.selector as String)]"
            case "Object" -> ".$(item.selector as String)"
        }
        ---
        if(isEmpty(accumulator))
            selectorExp
        else
           accumulator ++ selectorExp
     })

/**
* Returns `true` if the provided `Path` value is an `ATTRIBUTE_TYPE` expression.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | path | The `Path` value to validate.
* |===
*
* === Example
*
* This example shows how `isAttributeType` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Tree
* output application/json
* ---
* {
*   a: isAttributeType([{kind: OBJECT_TYPE, selector: "user", namespace: null},
*                       {kind: ATTRIBUTE_TYPE, selector: "name", namespace: null}]),
*   b: isAttributeType([{kind: OBJECT_TYPE, selector: "user", namespace: null},
*                       {kind: ARRAY_TYPE, selector: "name", namespace: null}]),
*   c: isAttributeType([{kind: ATTRIBUTE_TYPE, selector: "name", namespace: null}])
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "a": true,
*   "b": false,
*   "c": true
* }
* ----
**/
@Since(version = "2.4.0")
fun isAttributeType(path: Path): Boolean = do {
    path[-1].kind == ATTRIBUTE_TYPE
}

/**
* Returns `true` if the provided `Path` value is an `OBJECT_TYPE` expression.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | path | The `Path` value to validate.
* |===
*
* === Example
*
* This example shows how `isObjectType` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Tree
* output application/json
* ---
* {
*   a: isObjectType([{kind: OBJECT_TYPE, selector: "user", namespace: null},
*                    {kind: ATTRIBUTE_TYPE, selector: "name", namespace: null}]),
*   b: isObjectType([{kind: OBJECT_TYPE, selector: "user", namespace: null},
*                    {kind: OBJECT_TYPE, selector: "name", namespace: null}]),
*   c: isObjectType([{kind: OBJECT_TYPE, selector: "user", namespace: null}])
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
*   "c": true
* }
* ----
**/
@Since(version = "2.4.0")
fun isObjectType(path: Path): Boolean = do {
    path[-1].kind == OBJECT_TYPE
}

/**
* Returns `true` if the provided `Path` value is an `ARRAY_TYPE` expression.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | path | The `Path` value to validate.
* |===
*
* === Example
*
* This example shows how `isArrayType` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Tree
* output application/json
* ---
* {
*   a: isArrayType([{kind: OBJECT_TYPE, selector: "user", namespace: null},
*                   {kind: ATTRIBUTE_TYPE, selector: "name", namespace: null}]),
*   b: isArrayType([{kind: OBJECT_TYPE, selector: "user", namespace: null},
*                   {kind: ARRAY_TYPE, selector: 0, namespace: null}]),
*   c: isArrayType([{kind: ARRAY_TYPE, selector: 0, namespace: null}])
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
*   "c": true
* }
* ----
**/
@Since(version = "2.4.0")
fun isArrayType(path: Path): Boolean = do {
    path[-1].kind == ARRAY_TYPE
}


/**
* Maps the terminal (leaf) nodes in the tree.
*
*
* Leafs nodes cannot have an object or an array as a value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | The value to map.
* | callback | The mapper function.
* |===
*
* === Example
*
* This example transforms all the string values to upper case.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Tree
* output application/json
* ---
*  {
*      user: [{
*          name: "mariano",
*          lastName: "achaval"
*      }],
*      group: "data-weave"
*  } mapLeafValues (value, path) -> upper(value)
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "user": [
*      {
*        "name": "MARIANO",
*        "lastName": "ACHAVAL"
*      }
*    ],
*    "group": "DATA-WEAVE"
*  }
* ----
*
* === Example
*
* This example returns a new value for an object, array, or attribute.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* import * from dw::util::Tree
* ---
* {
*     name: "Mariano",
*     test: [1,2,3]
* } mapLeafValues ((value, path) -> if(isObjectType(path))
*                                         "***"
*                                   else if(isArrayType(path))
*                                         "In an array"
*                                   else "Is an attribute")
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "name": "***",
*   "test": [
*     "In an array",
*     "In an array",
*     "In an array"
*   ]
* }
* ----
**/
@Since(version = "2.2.2")
fun mapLeafValues(value: Any, callback: (value: Any, path: Path) -> Any): Any = do {

    fun doMapAttributes(key: Key, path: Path, callback: (value: Any, path: Path) -> Any) =
        key.@ mapObject (value, key) -> {
            (key) : doMapChildValues(value, path << {kind: ATTRIBUTE_TYPE, selector: key as String, namespace: key.#}, callback)
        }

    fun doMapChildValues(value: Any, path: Path, callback: (value: Any, path: Path) -> Any) = do {
        value match {
            case obj is  Object -> obj mapObject ((value, key, index) -> {
                (key)
                    @((doMapAttributes(key,path,callback))):
                        doMapChildValues(value, path << {kind: OBJECT_TYPE, selector: key as String, namespace: key.#}, callback)
            })
            case arr is Array -> arr map ((item, index) -> doMapChildValues(item, path << {kind: ARRAY_TYPE, selector: index, namespace: null}, callback))
            else -> callback(value, path)
        }
    }
    ---
    doMapChildValues(value, [], callback)
}


/**
* Filters the value or path of nodes in an input based on a
* specified `criteria`.
*
*
* The function iterates through the nodes in the input. The
* `criteria` can apply to the value or path in the input. If
* the `criteria` evaluates to `true`, the node remains in the
* output. If `false`, the function filters out the node.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | The value to filter.
* | criteria | The expression that determines whether to filter the node.
* |===
*
* === Example
*
* This example shows how `filterTree` behaves with different inputs.
* The output is `application/dw` for demonstration purposes.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Tree
* output application/dw
* ---
* {
*     a: {
*           name : "",
*           lastName @(foo: ""): "Achaval",
*           friends @(id: 123): [{id: "", test: true}, {age: 123}, ""]
*         } filterTree ((value, path) ->
*             value match  {
*                             case s is String -> !isEmpty(s)
*                             else -> true
*                           }
*     ),
*     b: null filterTree ((value, path) -> value is String),
*     c: [
*             {name: "Mariano", friends: []},
*             {test: [1,2,3]},
*             {dw: ""}
*         ] filterTree ((value, path) ->
*             value match  {
*                             case a is Array ->  !isEmpty(a as Array)
*                             else -> true
*                         })
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: {
*     lastName: "Achaval",
*     friends @(id: 123): [
*       {
*         test: true
*       },
*       {
*         age: 123
*       }
*     ]
*   },
*   b: null,
*   c: [
*     {
*       name: "Mariano"
*     },
*     {
*       test: [
*         1,
*         2,
*         3
*       ]
*     },
*     {
*       dw: ""
*     }
*   ]
* }
* ----
**/
@Since(version = "2.4.0")
fun filterTree(value: Any, criteria: (value: Any, path: Path) -> Boolean): Any = do {

    fun doFilterAttributes(key: Key, path: Path, callback: (value: Any, path: Path) -> Boolean): Object =
        (key.@ default {}) filterObject  ((value, key) ->
            callback(value, path << {kind: ATTRIBUTE_TYPE, selector: key as String, namespace: key.#})
        )

    fun doFilter(value: Any, path: Path, callback: (value: Any, path: Path) -> Boolean): Any = do {
        value match {
            case obj is Object -> do {
                obj
                    filterObject ((value, key, index) ->
                        callback(value, path << {kind: OBJECT_TYPE, selector: key as String, namespace: key.#})
                    )
                    mapObject ((value, key, index) -> {
                        (key) @((doFilterAttributes(key,path << {kind: OBJECT_TYPE, selector: key as String, namespace: key.#},callback))):
                                doFilter(value, path << {kind: OBJECT_TYPE, selector: key as String, namespace: key.#}, callback)
                               }
                    )

            }
            case arr is Array -> do {
                arr
                    filter ((item, index) -> callback(item, path << {kind: ARRAY_TYPE, selector: index, namespace: null}))
                    map ((item, index) ->
                        doFilter(item, path << {kind: ARRAY_TYPE, selector: index, namespace: null}, callback)
                    )
            }
            else -> value
        }
    }
    ---
    doFilter(value, [], criteria)
}


/**
* Applies a filtering expression to leaf or `Path` values of keys in
* an object.
*
*
* The leaf values in the object must be `SimpleType` or `Null` values. See
* https://docs.mulesoft.com/dataweave/latest/dw-core-types[Core Types]
* for descriptions of the types.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | An input value of `Any` type.
* | criteria | Boolean expression to apply to `SimpleType` or `Null`
*                leaf values of all objects in the input `value`. If the
*                result is `true`, the object retains the leaf value and
*                its key. If not, the function removes the leaf value
*                and key from the output.
* |===
*
* === Example
*
* This example shows how `filterObjectLeafs` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Tree
* var myArray = [{name @(mail: "me@me.com", test:123 ): "", id:"test"},
*                {name: "Me", id:null}]
* output application/json
* ---
* {
*  a: {
*      name: "Mariano",
*      lastName: null,
*      age: 123,
*      friends: myArray
*     }  filterObjectLeafs ((value, path) ->
*          !(value is Null or value is String)),
*  b: { c : null, d : "hello" } filterObjectLeafs ((value, path) ->
*          (value is Null and isObjectType(path)))
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "a": {
*     "age": 123,
*     "friends": [
*       {
*
*       },
*       {
*
*       }
*     ]
*   },
*   "b": {
*     "c": null
*   }
* }
* ----
**/
@Since(version = "2.4.0")
fun filterObjectLeafs(value: Any, criteria: (value: Any, path: Path) -> Boolean): Any = do {
    (value filterTree ((value, path) -> do {
        if((isObjectType(path) or isAttributeType(path)) and value is (SimpleType | Null))
            criteria(value, path)
        else
            true
    }))
}

/**
* Applies a filtering expression to leaf or `Path` values of an array.
*
*
* The leaf values in the array must be `SimpleType` or `Null` values. See
* https://docs.mulesoft.com/dataweave/latest/dw-core-types[Core Types]
* for descriptions of the types.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | An input value of `Any` type.
* | criteria | Boolean expression to apply to `SimpleType` or `Null`
*                leaf values of all arrays in the input `value`. If the
*                result is `true`, the array retains the leaf value. If
*                not, the function removes the leaf value from the output.
* |===
*
* === Example
*
* This example shows how `filterArrayLeafs` behaves with different
* inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Tree
* var myArray = [1, {name: ["", true], test: 213}, "123", null]
* output application/json
* ---
* {
*    a: myArray filterArrayLeafs ((value, path) ->
*         !(value is Null or value is String)),
*    b:  myArray filterArrayLeafs ((value, path) ->
*         (value is Null or value == 1)),
*    c: { a : [1,2] } filterArrayLeafs ((value, path) ->
*         (value is Null or value == 1)),
*    d: myArray filterArrayLeafs ((value, path) ->
*         !isArrayType(path))
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "a": [
*     1,
*     {
*       "name": [
*         true
*       ],
*       "test": 213
*     }
*   ],
*   "b": [
*     1,
*     {
*       "name": [
*
*       ],
*       "test": 213
*     },
*     null
*   ],
*   "c": {
*     "a": [
*      1
*     ]
*   },
*   "d": [
*     {
*       "name": [
*
*       ],
*       "test": 213
*     }
*   ]
* }
* ----
**/
@Since(version = "2.4.0")
fun filterArrayLeafs(value: Any, criteria: (value: Any, path: Path) -> Boolean): Any = do {
    (value filterTree ((value, path) -> do {
        if((isArrayType(path)) and value is (SimpleType | Null))
            criteria(value, path)
        else
            true
    }))
}


/**
* Returns `true` if any node in a given tree validates against
* the specified criteria.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | The value to search.
* | callback | The criteria to apply to the input `value`.
* |===
*
* === Example
*
* This example checks for each user by name and last name. Notice
* that you can also reference a `value` with `$` and the `path`
* with `$$`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Tree
* var myObject =  {
*      user: [{
*          name: "mariano",
*          lastName: "achaval",
*          friends: [
*              {
*                  name: "julian"
*              },
*              {
*                  name: "tom"
*              }
*          ]
*      },
*      {
*          name: "leandro",
*          lastName: "shokida",
*          friends: [
*              {
*                  name: "peter"
*              },
*              {
*                  name: "robert"
*              }
*          ]
*
*      }
*      ]
*  }
* output application/json
* ---
* {
*     mariano : myObject nodeExists ((value, path) -> path[-1].selector == "name" and value == "mariano"),
*     julian : myObject nodeExists ((value, path) -> path[-1].selector == "name" and value == "julian"),
*     tom : myObject nodeExists ($$[-1].selector == "name" and $ == "tom"),
*     leandro : myObject nodeExists ($$[-1].selector == "name" and $ ==  "leandro"),
*     peter : myObject nodeExists ($$[-1].selector == "name" and $ == "peter"),
*     wrongField: myObject nodeExists ($$[-1].selector == "wrongField"),
*     teo: myObject nodeExists ($$[-1].selector == "name" and $ == "teo")
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "mariano": true,
*   "julian": true,
*   "tom": true,
*   "leandro": true,
*   "peter": true,
*   "wrongField": false,
*   "teo": false
* }
* ----
**/
@Since(version = "2.2.2")
fun nodeExists(value: Any, callback: (value: Any, path: Path) -> Boolean):Boolean = do {

    fun existsInObject(value:Object,path: Path, callback: (value: Any, path: Path) -> Boolean):Boolean = do {
        value match {
            case {k:v ~ xs} -> do{
                var currentPath = path << {kind: OBJECT_TYPE, selector: k as String, namespace: k.#}
                var exists = callback(v, currentPath) or treeExists(v, currentPath, callback) or existsInAttribute(k.@ default {}, currentPath, callback)
                ---
                if(exists)
                    true
                else
                   treeExists(xs, path, callback)
            }
            case {} -> false
        }
    }

    fun existsInAttribute(value:Object, path: Path, callback: (value: Any, path: Path) -> Boolean):Boolean = do {
        value match {
            case {k:v ~ xs} -> do{
                var currentPath = path << {kind: ATTRIBUTE_TYPE, selector: k as String, namespace: k.#}
                var exists = callback(v, currentPath)
                ---
                if(exists)
                    true
                else
                   treeExists(xs, path, callback)
            }
            case {} -> false
        }
    }

    fun existsInArray(value:Array,path: Path,callback: (value: Any, path: Path) -> Boolean, index: Number):Boolean = do {
        value match {
            case [v ~ xs] -> do{
                var currentPath = path << {kind: ARRAY_TYPE, selector: index, namespace: null}
                var exists = callback(v, currentPath) or treeExists(v, currentPath, callback)
                ---
                if(exists)
                    true
                else
                   existsInArray(xs, path, callback, index + 1)
            }
            case [] -> false
        }
    }

    fun treeExists(value: Any, path: Path, callback: (value: Any, path: Path) -> Boolean):Boolean = do {
        value match {
            case obj is  Object -> existsInObject(obj,path,callback)
            case arr is Array -> existsInArray(arr,path,callback, 0)
            else -> callback(value, path) as Boolean
        }
    }
    ---
    treeExists(value, [], callback)
}
