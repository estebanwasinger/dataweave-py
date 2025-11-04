/**
* This module contains helper functions for working with numbers.
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::core::Numbers` to the header of your
* DataWeave script.
**/
@Since(version = "2.2.0")
%dw 2.0

/**
 * Transforms a decimal number into a hexadecimal number.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | number | The input number.
 * |===
 *
 * === Example
 *
 * This example shows how `toHex` behaves with different inputs.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import toHex from dw::core::Numbers
 * output application/json
 * ---
 * {
 *     a: toHex(-1),
 *     b: toHex(100000000000000000000000000000000000000000000000000000000000000),
 *     c: toHex(0),
 *     d: toHex(null),
 *     e: toHex(15),
 * }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * {
 *   "a": "-1",
 *   "b": "3e3aeb4ae1383562f4b82261d969f7ac94ca4000000000000000",
 *   "c": "0",
 *   "d": null,
 *   "e": "f"
 * }
 * ----
 */
@Since(version = "2.2.0")
fun toHex(number: Number): String = toRadixNumber(number, 16)

/**
* Helper function that enables `toHex` to work with null value.
*/
@Since(version = "2.2.0")
fun toHex(number: Null): Null = null

/**
 * Transforms a decimal number into a binary number.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | number | The input number.
 * |===
 *
 * === Example
 *
 * This example shows how the `toBinary` behaves with different inputs.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import toBinary from dw::core::Numbers
 * output application/json
 * ---
 * {
 *     a: toBinary(-2),
 *     b: toBinary(100000000000000000000000000000000000000000000000000000000000000),
 *     c: toBinary(0),
 *     d: toBinary(null),
 *     e: toBinary(2),
 * }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * {
 *   "a": "-10",
 *   "b": "11111000111010111010110100101011100001001110000011010101100010111101001011100000100010011000011101100101101001111101111010110010010100110010100100000000000000000000000000000000000000000000000000000000000000",
 *   "c": "0",
 *   "d": null,
 *   "e": "10"
 * }
 * ----
 */
@Since(version = "2.2.0")
fun toBinary(number: Number): String = toRadixNumber(number, 2)

/**
* Helper function that enables `toBinary` to work with null value.
*/
@Since(version = "2.2.0")
fun toBinary(number: Null): Null = null

/**
 * Transforms a hexadecimal number into decimal number.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | hexText | The hexadecimal number represented in a `String`.
 * |===
 *
 * === Example
 *
 * This example shows how the `toBinary` behaves with different inputs.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import fromHex from dw::core::Numbers
 * output application/json
 * ---
 * {
 *     a: fromHex("-1"),
 *     b: fromHex("3e3aeb4ae1383562f4b82261d969f7ac94ca4000000000000000"),
 *     c: fromHex(0),
 *     d: fromHex(null),
 *     e: fromHex("f"),
 * }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * {
 *   "a": -1,
 *   "b": 100000000000000000000000000000000000000000000000000000000000000,
 *   "c": 0,
 *   "d": null,
 *   "e": 15
 * }
 * ----
 */
@Since(version = "2.2.0")
fun fromHex(hexText: String): Number = fromRadixNumber(hexText, 16)

/**
* Helper function that enables `fromHex` to work with null value.
*/
@Since(version = "2.2.0")
fun fromHex(hexText: Null): Null = null

/**
 * Transforms from a binary number into a decimal number.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | binaryText | The binary number represented in a `String`.
 * |===
 *
 * === Example
 *
 * This example shows how the `toBinary` behaves with different inputs.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import fromBinary from dw::core::Numbers
 * output application/json
 * ---
 * {
 *     a: fromBinary("-10"),
 *     b: fromBinary("11111000111010111010110100101011100001001110000011010101100010111101001011100000100010011000011101100101101001111101111010110010010100110010100100000000000000000000000000000000000000000000000000000000000000"),
 *     c: fromBinary(0),
 *     d: fromBinary(null),
 *     e: fromBinary("100"),
 * }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * {
 *   "a": -2,
 *   "b": 100000000000000000000000000000000000000000000000000000000000000,
 *   "c": 0,
 *   "d": null,
 *   "e": 4
 * }
 * ----
 */
@Since(version = "2.2.0")
fun fromBinary(binaryText: String): Number = fromRadixNumber(binaryText, 2)

/**
* Helper function that enables `fromBinary` to work with null value.
*/
@Since(version = "2.2.0")
fun fromBinary(binaryText: Null): Null = null

/**
 * Transforms a number in the specified radix into decimal number
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | numberText | The number text.
 * | radix | The radix number.
 * |===
 *
 * === Example
 *
 * This example shows how the `fromRadixNumber` behaves under different inputs.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import fromRadixNumber from dw::core::Numbers
 * output application/json
 * ---
 * {
 *     a: fromRadixNumber("10", 2),
 *     b: fromRadixNumber("FF", 16)
 * }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * {
 *   "a": 2,
 *   "b": 255
 * }
 * ----
 */
@Since(version = "2.2.0")
fun fromRadixNumber(numberStr: String, radix: Number): Number = native("system::StringWithRadixToNumber")

/**
 * Transforms a decimal number into a number string in other radix.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | number | The decimal number.
 * | radix | The radix of the result number.
 * |===
 *
 * === Example
 *
 * This example shows how the `toRadixNumber` behaves under different inputs.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import toRadixNumber from dw::core::Numbers
 * output application/json
 * ---
 * {
 *     a: toRadixNumber(2, 2),
 *     b: toRadixNumber(255, 16)
 * }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * {
 *   "a": "10",
 *   "b": "ff"
 * }
 * ----
 */
@Since(version = "2.2.0")
fun toRadixNumber(number: Number, radix: Number): String = native("system::NumberToRadixFunction")
