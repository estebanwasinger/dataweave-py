/**
* This module contains helper functions for working with arrays.
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::core::Arrays` to the header of your
* DataWeave script.
*/
%dw 2.0

/**
 * Returns `true` if at least one element in the array matches the specified condition.
 *
 * 
 * The function stops iterating after the first element that matches the condition is found.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | list | The input array.
 * | condition | A condition (or expression) used to match elements in the array.
 * |===
 *
 * === Example
 *
 * This example applies a variety of expressions to elements of several input arrays.
 * The `&#36;` in the condition is the default parameter for the current element of the
 * array that the condition evaluates.
 * Note that you can replace the default `&#36;` parameter with a lambda expression that
 * contains a named parameter for the current array element.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * output application/json
 * ---
 * { "results" : [
 *     "ok" : [
 *       [1,2,3] some (($ mod 2) == 0),
 *       [1,2,3] some ((nextNum) -> (nextNum mod 2) == 0),
 *       [1,2,3] some (($ mod 2) == 1),
 *       [1,2,3,4,5,6,7,8] some (log('should stop at 2 ==', $) == 2),
 *       [1,2,3] some ($ == 1),
 *       [1,1,1] some ($ == 1),
 *       [1] some ($ == 1)
 *     ],
 *     "err" : [
 *       [1,2,3] some ($ == 100),
 *       [1] some ($ == 2)
 *     ]
 *   ]
 * }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * {
 *    "results": [
 *      {
 *        "ok": [ true, true, true, true, true, true, true ]
 *      },
 *      {
 *        "err": [ false, false ]
 *      }
 *    ]
 *  }
 * ----
 */
@Labels(labels =["exits"])
fun some<T>(list: Array<T>, condition: (T) -> Boolean): Boolean = do {
    @TailRec
    fun internalSome<T>(list: Array<T>, condition: (T) -> Boolean): Boolean =
        list match {
            case [] -> false
            case [head ~ tail] ->
              if(condition(head))
                true
              else
                internalSome(tail, condition)
        }
    ---
    internalSome(list, condition)
}


/**
* Helper function that enables `some` to work with a `null` value.
*/
@Since(version="2.3.0")
@Labels(labels =["exits"])
fun some(list: Null, condition: (Nothing) -> Any): Boolean = false

/**
 * Returns `true` if every element in the array matches the condition.
 *
 *
 * The function stops iterating after the first negative evaluation of an
 * element in the array.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | list | The input array.
 * | condition | A condition (or expression) to apply to elements in the input array.
 * |===
 *
 * === Example
 *
 * This example applies a variety of expressions to the input arrays. The `$`
 * references values of the elements.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * var arr0 = [] as Array<Number>
 * output application/json
 * ---
 * { "results" : [
 *      "ok" : [
 *         [1,1,1] every ($ == 1),
 *         [1] every ($ == 1)
 *      ],
 *      "err" : [
 *         [1,2,3] every ((log('should stop at 2 ==', $) mod 2) == 1),
 *         [1,1,0] every ($ == 1),
 *         [0,1,1,0] every (log('should stop at 0 ==', $) == 1),
 *         [1,2,3] every ($ == 1),
 *         arr0 every true,
 *      ]
 *    ]
 *  }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * {
 *    "results": [
 *      {
 *        "ok": [ true, true ]
 *      },
 *      {
 *        "err": [ false, false, false, false, false ]
 *      }
 *    ]
 *  }
 * ----
 */
@Labels(labels =["forAll"])
fun every<T>(list: Array<T>, condition: (T) -> Boolean): Boolean = do {
  @TailRec
  fun internalEvery<T>(list: Array<T>, condition: (T) -> Boolean): Boolean =
    list match {
        case [] -> true
        case [head ~ tail] ->
         if(condition(head))
           internalEvery(tail, condition)
         else
           false
      }
  ---
  internalEvery(list, condition)

}

/**
* Helper function that enables `every` to work with a `null` value.
*/
@Since(version="2.3.0")
@Labels(labels =["forAll"])
fun every(value: Null, condition: (Nothing) -> Any): Boolean = true

/**
 * Counts the elements in an array that return `true` when the matching function is applied to the value of each element.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | array | The input array that contains elements to match.
 * | matchingFunction | A function to apply to elements in the input array.
 * |===
 *
 * === Example
 *
 * This example counts the number of elements in the input array ([1, 2, 3, 4]) that
 * return `true` when the function `(($ mod 2) == 0)` is applied their values. In this
 * case, the values of _two_ of the elements, both `2` and `4`, match because
 * `2 mod 2 == 0` and `4 mod 2 == 0`. As a consequence, the `countBy` function returns `2`.
 * Note that `mod` returns the modulus of the operands.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * output application/json
 * ---
 * { "countBy" : [1, 2, 3, 4] countBy (($ mod 2) == 0) }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * { "countBy": 2 }
 * ----
 */
fun countBy<T>(@StreamCapable array: Array<T>, matchingFunction: (T) -> Boolean): Number = do {
    if (isDefaultOperatorDisabledExceptionHandling())
     array match {
       case [] -> 0
       else ->
         ($ reduce (item: T, carry: Number = 0) ->
           if (matchingFunction(item))
             carry + 1
           else
             carry)
    }
    else
      (array reduce (item: T, carry: Number = 0) ->
        if (matchingFunction(item))
          carry + 1
        else
         carry) default 0
}

/**
* Helper function that enables `countBy` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun countBy(array: Null, matchingFunction: (Nothing) -> Any): Null = null

/**
 * Returns the sum of the values of the elements in an array.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | array | The input array.
 * | numberSelector | A DataWeave selector that selects the values of the numbers in the input array.
 * |===
 *
 * === Example
 *
 * This example calculates the sum of the values of elements some arrays. Notice
 * that both of the `sumBy` function calls produce the same result.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * output application/json
 * ---
 * {
 *   "sumBy" : [
 *     [ { a: 1 }, { a: 2 }, { a: 3 } ] sumBy $.a,
 *     sumBy([ { a: 1 }, { a: 2 }, { a: 3 } ], (item) -> item.a)
 *   ]
 * }
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * { "sumBy" : [ 6, 6 ] }
 * ----
 */
fun sumBy<T>(@StreamCapable array: Array<T>, numberSelector: (T) -> Number): Number = do {
  if (isDefaultOperatorDisabledExceptionHandling())
    array match {
      case [] -> 0
      else ->
        ($ reduce (item: T, carry: Number = 0) -> numberSelector(item) + carry)
    }
  else
    (array reduce (item: T, carry: Number = 0) -> numberSelector(item) + carry) default 0
}

/**
* Helper function that enables `sumBy` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun sumBy(array: Null, numberSelector: (Nothing) -> Any): Null = null

/**
* Breaks up an array into sub-arrays that contain the
* specified number of elements.
*
*
* When there are fewer elements in the input array than the specified number,
* the function fills the sub-array with those elements. When there are more
* elements, the function fills as many sub-arrays needed with the extra
* elements.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | items | Items in the input array.
* | amount | The number of elements allowed per sub-array.
* |===
*
* === Example
*
* This example breaks up arrays into sub-arrays based on the specified `amount`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Arrays
* output application/json
* ---
* {
*   "divideBy" : [
*       { "divideBy2" : [1, 2, 3, 4, 5] divideBy 2 },
*       { "divideBy2" : [1, 2, 3, 4, 5, 6] divideBy 2 },
*       { "divideBy3" : [1, 2, 3, 4, 5] divideBy 3 }
*   ]
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*  "divideBy": [
*   {
*     "divideBy2": [
*       [ 1, 2 ],
*       [ 3, 4 ],
*       [ 5 ]
*     ]
*   },
*   {
*     "divideBy2": [
*       [ 1, 2 ],
*       [ 3, 4 ],
*       [ 5, 6 ]
*     ]
*   },
*     {
*       "divideBy3": [
*         [ 1, 2, 3 ],
*         [ 4, 5 ]
*       ]
*     }
*  ]
* }
* ----
*/
@Labels(labels =["split", "group"])
fun divideBy<T>(items: Array<T>, amount: Number): Array<Array<T>> = do {
    fun internalDivideBy<T>(items: Array<T>, amount: Number, carry: Array<T>, remaining: Number ): Array<Array<T>> =
      items match {
          case [x ~ xs] ->
            if(remaining <= 1)
                [carry << x ~ internalDivideBy(xs, amount, [], amount)]
            else
               internalDivideBy(xs, amount, carry << x , remaining - 1)
          else ->
            if(isEmpty(carry))
             []
            else
             [carry]
      }
    ---
    internalDivideBy(items, amount, [], amount)
}

/**
* Helper function that enables `divideBy` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun divideBy(items: Null, amount: Any): Null = null

/**
 * Joins two arrays of objects by a given ID criteria.
 *
 *
 * `join` returns an array all the `left` items, merged by ID with any
 * right items that exist.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | left | The left-side array of objects.
 * | right | The right-side array of objects.
 * | leftCriteria | The criteria used to extract the ID for the left collection.
 * | rightCriteria | The criteria used to extract the ID for the right collection.
 * |===
 *
 * === Example
 *
 * This example shows how join behaves. Notice that the output only includes
 * objects where the values of the input `user.id` and `product.ownerId` match.
 * The function includes the `"l"` and `"r"` keys in the output.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * var users = [{id: "1", name:"Mariano"},{id: "2", name:"Leandro"},{id: "3", name:"Julian"},{id: "5", name:"Julian"}]
 * var products = [{ownerId: "1", name:"DataWeave"},{ownerId: "1", name:"BAT"}, {ownerId: "3", name:"DataSense"}, {ownerId: "4", name:"SmartConnectors"}]
 * output application/json
 * ---
 * join(users, products, (user) -> user.id, (product) -> product.ownerId)
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * [
 *   {
 *     "l": {
 *       "id": "1",
 *       "name": "Mariano"
 *     },
 *     "r": {
 *       "ownerId": "1",
 *       "name": "DataWeave"
 *     }
 *   },
 *   {
 *     "l": {
 *       "id": "1",
 *       "name": "Mariano"
 *     },
 *     "r": {
 *       "ownerId": "1",
 *       "name": "BAT"
 *     }
 *   },
 *   {
 *     "l": {
 *       "id": "3",
 *       "name": "Julian"
 *     },
 *     "r": {
 *       "ownerId": "3",
 *       "name": "DataSense"
 *     }
 *   }
 * ]
 * ----
 */
@Since(version = "2.2.0")
fun join<L <: {}, R <: {}>(left:Array<L>, right:Array<R>, leftCriteria: (leftValue: L) -> String, rightCriteria: (rightValue: R) -> String): Array<Pair<L, R>> = do {
    var groupedBy = right groupBy ((r) -> rightCriteria(r))
    ---
    left flatMap ((lValue, index) -> do {
        var leftValue = leftCriteria(lValue) as String
        var value = groupedBy[leftValue] default []
        ---
        value map ((rValue, index) ->
            {l: lValue, r: rValue}
        )
    })
}


/**
 * Joins two arrays of objects by a given ID criteria.
 *
 *
 * `leftJoin` returns an array all the `left` items, merged by ID with any right
 * items that meet the joining criteria.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | left | The left-side array of objects.
 * | right | The right-side array of objects.
 * | leftCriteria | The criteria used to extract the ID for the left collection.
 * | rightCriteria | The criteria used to extract the ID for the right collection.
 * |===
 *
 * === Example
 *
 * This example shows how join behaves. Notice that it returns all objects from
 * the left-side array (`left`) but only joins items from the right-side array
 * (`right`) if the values of the left-side `user.id` and right-side
 * `product.ownerId` match.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * var users = [{id: "1", name:"Mariano"},{id: "2", name:"Leandro"},{id: "3", name:"Julian"},{id: "5", name:"Julian"}]
 * var products = [{ownerId: "1", name:"DataWeave"},{ownerId: "1", name:"BAT"}, {ownerId: "3", name:"DataSense"}, {ownerId: "4", name:"SmartConnectors"}]
 * output application/json
 * ---
 * leftJoin(users, products, (user) -> user.id, (product) -> product.ownerId)
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * [
 *   {
 *     "l": {
 *       "id": "1",
 *       "name": "Mariano"
 *     },
 *     "r": {
 *       "ownerId": "1",
 *       "name": "DataWeave"
 *     }
 *   },
 *   {
 *     "l": {
 *       "id": "1",
 *       "name": "Mariano"
 *     },
 *     "r": {
 *       "ownerId": "1",
 *       "name": "BAT"
 *     }
 *   },
 *   {
 *     "l": {
 *       "id": "2",
 *       "name": "Leandro"
 *     }
 *   },
 *   {
 *     "l": {
 *       "id": "3",
 *       "name": "Julian"
 *     },
 *     "r": {
 *       "ownerId": "3",
 *       "name": "DataSense"
 *     }
 *   },
 *   {
 *     "l": {
 *       "id": "5",
 *       "name": "Julian"
 *     }
 *   }
 * ]
 * ----
 */
@Since(version = "2.2.0")
fun leftJoin<L <: {}, R <: {}>(left:Array<L>, right:Array<R>, leftCriteria: (leftValue: L) -> String, rightCriteria: (rightValue: R) -> String): Array<{l: L,r?: R}> = do {
    var groupedBy = right groupBy ((r) -> rightCriteria(r))
    ---
    left flatMap ((lValue, index) -> do {
        var leftValue = leftCriteria(lValue) as String
        var value = groupedBy[leftValue] default []
        ---
        value match {
            case [] -> [{l:lValue}]
            else ->
                value map ((rValue, index) ->
                    {l:lValue , r:rValue}
                )
        }
    })
}

/**
 * Joins two array of objects by a given `ID` criteria.
 *
 *
 * `outerJoin` returns an array with all the `left` items, merged by ID
 * with the `right` items in cases where any exist, and it returns `right`
 * items that are not present in the `left`.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | left | The left-side array of objects.
 * | right | The right-side array of objects.
 * | leftCriteria | The criteria used to extract the ID for the left collection.
 * | rightCriteria | The criteria used to extract the ID for the right collection.
 * |===
 *
 * === Example
 *
 * This example shows how join behaves. Notice that the output includes
 * objects where the values of the input `user.id` and `product.ownerId` match,
 * and it includes objects where there is no match for the value of the
 * `user.id` or `product.ownerId`.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * var users = [{id: "1", name:"Mariano"},{id: "2", name:"Leandro"},{id: "3", name:"Julian"},{id: "5", name:"Julian"}]
 * var products = [{ownerId: "1", name:"DataWeave"},{ownerId: "1", name:"BAT"}, {ownerId: "3", name:"DataSense"}, {ownerId: "4", name:"SmartConnectors"}]
 * output application/json
 * ---
 * outerJoin(users, products, (user) -> user.id, (product) -> product.ownerId)
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * [
 *   {
 *     "l": {
 *       "id": "1",
 *       "name": "Mariano"
 *     },
 *     "r": {
 *       "ownerId": "1",
 *       "name": "DataWeave"
 *     }
 *   },
 *   {
 *     "l": {
 *       "id": "1",
 *       "name": "Mariano"
 *     },
 *     "r": {
 *       "ownerId": "1",
 *       "name": "BAT"
 *     }
 *   },
 *   {
 *     "l": {
 *       "id": "2",
 *       "name": "Leandro"
 *     }
 *   },
 *   {
 *     "l": {
 *       "id": "3",
 *       "name": "Julian"
 *     },
 *     "r": {
 *       "ownerId": "3",
 *       "name": "DataSense"
 *     }
 *   },
 *   {
 *     "l": {
 *       "id": "5",
 *       "name": "Julian"
 *     }
 *   },
 *   {
 *     "r": {
 *       "ownerId": "4",
 *       "name": "SmartConnectors"
 *     }
 *   }
 * ]
 * ----
 */
@Since(version = "2.2.0")
fun outerJoin<L <: {}, R <: {}>(left:Array<L>, right:Array<R>, leftCriteria: (leftValue: L) -> String, rightCriteria: (rightValue: R) -> String): Array<{l?: L,r?: R}> = do {
    var leftGroupBy = left groupBy (r) -> leftCriteria(r)
    ---
    leftJoin(left,right,leftCriteria,rightCriteria) ++  do {
        right
            filter ((rValue, index) -> !leftGroupBy[rightCriteria(rValue) as String]?)
            map ((rightValue) ->
                  {
                    r: rightValue
                  }
                )
    }
}

/**
 * Selects the first `n` elements. It returns an empty array when `n &lt;= 0`
 * and the original array when `n > sizeOf(array)`.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | array | The array of elements.
 * | n | The number of elements to select.
 * |===
 *
 * === Example
 *
 * This example outputs an array that contains the values of first two elements
 * of the input array.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * var users = ["Mariano", "Leandro", "Julian"]
 * output application/json
 * ---
 * take(users, 2)
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * [
 *   "Mariano",
 *   "Leandro"
 * ]
 * ----
 */
@Since(version = "2.2.0")
fun take<T>(array: Array<T>, n: Number): Array<T> = do {
  @TailRec()
  fun doTake(array: Array<T>, current: Number, accum: Array<T>): Array<T> =
    array match {
      case [] -> accum
      case [head ~ tail] ->
        if (current < n) doTake(tail, current + 1, accum << head) else accum
    }
  ---
  if (n <= 0) [] else doTake(array, 0, [])
}

/**
* Helper function that enables `take` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun take(array: Null, n: Any): Null = null

/**
 * Drops the first `n` elements. It returns the original array when `n &lt;= 0`
 * and an empty array when `n > sizeOf(array)`.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | array | The left array of elements.
 * | n | The number of elements to take.
 * |===
 *
 * === Example
 *
 * This example returns an array that only contains the third element of the
 * input array. It drops the first two elements from the output.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * var users = ["Mariano", "Leandro", "Julian"]
 * output application/json
 * ---
 * drop(users, 2)
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * [
 *   "Julian"
 * ]
 * ----
 */
@Since(version = "2.2.0")
fun drop<T>(array: Array<T>, n: Number): Array<T> = do {
    if (n <= 0)
        array
    else
        array[n to -1] default []
}

/**
* Helper function that enables `drop` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun drop(array: Null, n: Any): Null = null

/**
 * Selects the interval of elements that satisfy the condition:
 * `from &lt;= indexOf(array) < until`
 *
 * === Parameters
 * 
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | array | The array of elements.
 * | from | The starting index of the interval of elements to include from the array. +
 * If this value is negative, the function starts including from the first element of the array. If this value is higher than the last index of the array, the function returns an empty array (`[]`).
 * | until | The ending index of the interval of elements to include from the array. +
 * If this value is higher than the last index of the array, the function includes up to the last element of the array. If this value is lower than the first index of the array, the function returns an empty array (`[]`).
 * |===
 *
 * === Example
 *
 * This example returns an array that contains the values of indices
 * 1, 2, and 3 from the input array. It excludes the values of indices
 * 0, 4, and 5.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * output application/json
 * var arr = [0,1,2,3,4,5]
 * ---
 * slice(arr, 1, 4)
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * [
 *   1,
 *   2,
 *   3
 * ]
 * ----
 */
@Since(version = "2.2.0")
fun slice<T>(array: Array<T>, from: Number, until: Number): Array<T> = do {
    if (from < 0)
        slice(array, 0, until)
    else if (from >= until)
        []
    else
        array[from to (until-1)] default array[from to -1] default []
}

/**
* Helper function that enables `slice` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun slice(array: Null, from: Any, until: Any): Null = null

/**
 * Returns the index of the first occurrence of an element within the array. If the value is not found, the function returns `-1`.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | array | The array of elements.
 * | toFind | The element to find.
 * |===
 *
 * === Example
 *
 * This example returns the index of the matching value from the input array.
 * The index of `"Julian"` is `2`.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * output application/json
 * var users = ["Mariano", "Leandro", "Julian"]
 * ---
 * indexOf(users, "Julian")
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * 2
 * ----
 *
 */
@Since(version = "2.2.0")
fun indexOf<T>(array: Array<T>, toFind: T): Number = do {
    dw::Core::indexOf(array, toFind)
}

/**
 * Returns the index of the first occurrence of an element that matches a
 * condition within the array. If no element matches the condition, the function returns `-1`.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | array | The array of elements.
 * | condition | The condition (or expression) used to match an element in the array.
 * |===
 *
 * === Example
 *
 * This example returns the index of the value from the input array that
 * matches the condition in the lambda expression,
 * `(item) -> item startsWith "Jul"`.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * output application/json
 * var users = ["Mariano", "Leandro", "Julian"]
 * ---
 * users indexWhere (item) -> item startsWith "Jul"
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * 2
 * ----
 */
@Since(version = "2.2.0")
fun indexWhere<T>(array: Array<T>, condition: (item: T) -> Boolean): Number = do {
  fun private_indexWhere(arr: Array<T>, currIndex: Number) = do {
    arr match {
      case [] -> -1
      case [head ~ tail] ->
        if (condition(head))
          currIndex
        else
          private_indexWhere(tail, currIndex+1)
    }
  }
  ---
  private_indexWhere(array, 0)
}

/**
* Helper function that enables `indexWhere` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun indexWhere(array: Null, condition: (item: Nothing) -> Any): Null = null

/**
 * Splits an array into two at a given position.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | array | The array of elements.
 * | n | The index at which to split the array.
 * |===
 *
 * === Example
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * output application/json
 * var users = ["Mariano", "Leandro", "Julian"]
 * ---
 * users splitAt 1
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * {
 *   "l": [
 *     "Mariano"
 *   ],
 *   "r": [
 *     "Leandro",
 *     "Julian"
 *   ]
 * }
 * ----
 */
@Since(version = "2.2.0")
fun splitAt<T>(array: Array<T>, n: Number): Pair<Array<T>, Array<T>> = {
  l: array take n,
  r: array drop n
}

/**
* Helper function that enables `splitAt` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun splitAt(array: Null, n: Any): Null = null

/**
 * Selects elements from the array while the condition is met but
 * stops the selection process when it reaches an element that
 * fails to satisfy the condition.
 *
 *
 * To select all elements that meet the condition, use the `filter` function.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | array | The array of elements.
 * | condition | The condition (or expression) used to match an element in the array.
 * |===
 *
 * === Example
 *
 * This example iterates over the elements in the array and selects only those
 * with an index that is `&lt;= 1` and stops selecting elements when it reaches
 * one that is greater than `2`. Notice that it does not select the second `1` because
 * of the `2` that precedes it in the array. The function outputs the result into an array.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * output application/json
 * var arr = [0,1,2,1]
 * ---
 * arr takeWhile $ <= 1
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * [
 *   0,
 *   1
 * ]
 * ----
 */
@Since(version = "2.2.0")
fun takeWhile<T>(array: Array<T>, condition: (item: T) -> Boolean): Array<T> = do {
  array match {
    case [] -> array
    case [head ~ tail] ->
      if (condition(head))
        [head ~ takeWhile(tail, condition)]
      else
        []
  }
}

/**
* Helper function that enables `takeWhile` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun takeWhile(array: Null, condition: (item: Nothing) -> Any): Null = null

/**
 * Drops elements from the array while the condition is met but stops the selection process
 * when it reaches an element that fails to satisfy the condition.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | array | The array of elements.
 * | condition | The condition (or expression) used to match an element in the array.
 * |===
 *
 * === Example
 *
 * This example returns an array that omits elements that are less than or equal to `2`.
 * The last two elements (`2` and `1`) are included in the output array because the
 * function stops dropping elements when it reaches the `3`, which is greater than `2`.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * output application/json
 * var arr = [0,1,3,2,1]
 * ---
 * arr dropWhile $ < 3
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * [
 *   3,
 *   2,
 *   1
 * ]
 * ----
 */
@Since(version = "2.2.0")
fun dropWhile<T>(array: Array<T>, condition: (item: T) -> Boolean): Array<T> = do {
  array match {
    case [] -> array
    case [head ~ tail] ->
      if (condition(head))
        dropWhile(tail, condition)
      else
        array
  }
}

/**
* Helper function that enables `dropWhile` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun dropWhile(array: Null, condition: (item: Nothing) -> Any): Null = null

/**
 * Splits an array into two at the first position where the condition is met.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | array | The array of elements to split.
 * | condition | The condition (or expression) used to match an element in the array.
 * |===
 *
 * === Example
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * output application/json
 * var users = ["Mariano", "Leandro", "Julian", "Tomo"]
 * ---
 * users splitWhere (item) -> item startsWith "Jul"
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * {
 *   "l": [
 *     "Mariano",
 *     "Leandro"
 *   ],
 *   "r": [
 *     "Julian",
 *     "Tomo"
 *   ]
 * }
 * ----
 */
@Since(version = "2.2.0")
fun splitWhere<T>(array: Array<T>, condition: (item: T) -> Boolean): Pair<Array<T>, Array<T>> = do {
  var index = array indexWhere (item) -> condition(item)
  ---
  array splitAt index
}

/**
* Helper function that enables `splitWhere` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun splitWhere(array: Null, condition: (item: Nothing) -> Any): Null = null

/**
 * Separates the array into the elements that satisfy the condition from those
 * that do not.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | array | The array of elements to split.
 * | condition | The condition (or expression) used to match an element in the array.
 * |===
 *
 * === Example
 *
 * This example partitions numbers found within an input array. The
 * even numbers match the criteria set by the lambda expression
 * `(item) -> isEven(item)`. The odd do not. The function generates the
 * `"success"` and `"failure"` keys within the output object.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Arrays
 * output application/json
 * var arr = [0,1,2,3,4,5]
 * ---
 * arr partition (item) -> isEven(item)
 * ----
 *
 * ==== Output
 *
 * [source,json,linenums]
 * ----
 * {
 *   "success": [
 *     0,
 *     2,
 *     4
 *   ],
 *   "failure": [
 *     1,
 *     3,
 *     5
 *   ]
 * }
 * ----
 */
@Since(version = "2.2.0")
fun partition<T>(array: Array<T>, condition: (item: T) -> Boolean): {success: Array<T>, failure: Array<T>} = {
  success: array filter (item) -> condition(item),
  failure: array filter (item) -> !condition(item)
}

/**
* Helper function that enables `partition` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun partition(array: Null, condition: (item: Nothing) -> Any): Null = null

/**
* Returns the first element that satisfies the condition, or returns `null` if no
* element meets the condition.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | array | The array of elements to search.
* | condition | The condition to satisfy.
* |===
*
* === Example
*
* This example shows how `firstWith` behaves when an element matches and when an element does not match.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* import firstWith from dw::core::Arrays
* var users = [{name: "Mariano", lastName: "Achaval"}, {name: "Ana", lastName: "Felisatti"}, {name: "Mariano", lastName: "de Sousa"}]
* ---
* {
*   a: users firstWith ((user, index) -> user.name == "Mariano"),
*   b: users firstWith ((user, index) -> user.name == "Peter")
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "a": {
*     "name": "Mariano",
*     "lastName": "Achaval"
*   },
*   "b": null
* }
* ----
**/
@Since(version = "2.3.0")
fun firstWith<T>(array:Array<T>, condition: (item: T, index: Number) ->  Boolean): T | Null = do {
    filter(array, condition)[0]
}

/**
* Helper function that enables `firstWith` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun firstWith(array: Null, condition: (item: Nothing, index: Nothing) -> Any): Null = null
