/**
* This module contains helper functions for working with binaries.
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::core::Binaries` to the header of your
* DataWeave script.
*/
%dw 2.0

/**
 * Transforms a binary value into a hexadecimal string.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | content | The `Binary` value to transform.
 * |===
 *
 * === Example
 *
 * This example transforms a binary version of "Mule" (defined in the variable,
 * `myBinary`) to hexadecimal.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Binaries
 * output application/json
 * var myBinary = "Mule" as Binary
 * var testType = typeOf(myBinary)
 * ---
 * {
 *    "binaryToHex" : toHex(myBinary)
 * }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * { "binaryToHex": "4D756C65" }
 * ----
 */
fun toHex(content: Binary): String = content as String {base: "16"}

/**
 * Transforms a hexadecimal string into a binary.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | hexString | A hexadecimal string to transform.
 * |===
 *
 * === Example
 *
 * This example transforms a hexadecimal string to "Mule".
 * To make the resulting type clear, it outputs data in the `application/dw` format.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Binaries
 * output application/dw
 * ---
 * { "hexToBinary": fromHex("4D756C65") }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * {
 *    hexToBinary: "TXVsZQ==" as Binary {base: "64"}
 * }
 * ----
 */
fun fromHex(hexString: String): Binary = hexString as Binary {base: "16"}

/**
 * Transforms a binary value into a Base64 string.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | content | The binary value to transform.
 * |===
 *
 * === Example
 *
 * This example transforms a binary value into a Base64 encoded string. In this case, the binary value represents an image.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 *
 * import dw::Crypto
 * import toBase64 from dw::core::Binaries
 *
 * var emailChecksum = Crypto::MD5("achaval@gmail.com" as Binary)
 * var image = readUrl(log("https://www.gravatar.com/avatar/$(emailChecksum)"), "application/octet-stream")
 *
 * output application/json
 * ---
 * toBase64(image)
 * ----
 *
 * ==== Output
 *
 * This example outputs a Base64 encoded string. The resulting string was shortened for readability purposes:
 *
 * [source,JSON,linenums]
 * ----
 * "/9j/4AAQSkZJRgABAQEAYABgAAD//..."
 * ----
 */
fun toBase64(content: Binary): String =
    // We need to do the double cast for compatibility issue to avoid carrying the base information
    content as String {base: "64"} as String

/**
 * Transforms a Base64 string into a binary value.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | base64String | The Base64 string to transform.
 * |===
 *
 * === Example
 *
 * This example takes a Base64 encoded string and transforms it into a binary value. This example assumes that the `payload` contains the Base64 string generated from an image in example xref:dw-binaries-functions-tobase64.adoc#toBase64-example[toBase64].
 * The output of this function is a binary value that represents the image generated in example https://docs.mulesoft.com/dataweave/latest/dw-binaries-functions-tobase64[toBase64].
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::core::Binaries
 * output application/octet-stream
 * ---
 * fromBase64(payload)
 * ----
 *
 */
fun fromBase64(base64String: String): Binary =
     // We need to do the double cast for compatibility issue to avoid carrying the base information
    base64String as Binary {base: "64"} as Binary

/**
* Splits the specified binary content into lines and returns the results in an
* array.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | content | Binary data to read and split.
* | charset | String representing the encoding to read.
* |===
*
* === Example
*
* This example transforms binary content, which is separated into new
* lines (`\n`), in a comma-separated array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Binaries
* var content = read("Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n", "application/octet-stream")
* output application/json
* ---
* {
*    lines : (content readLinesWith "UTF-8"),
*    showType: typeOf(content)
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*    "lines": [ "Line 1", "Line 2", "Line 3", "Line 4", "Line 5" ],
*    "showType": "Binary"
* }
* ----
*/
@Since(version = "2.2.0")
fun readLinesWith(content: Binary, charset: String): Array<String> = native("system::ReadLinesFunctionValue")

/**
* Writes the specified lines and returns the binary content.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | content | Array of items to write.
* | charset | String representing the encoding to use when writing.
* |===
*
* === Example
*
* This example inserts a new line (`\n`) after each iteration. Specifically,
* it uses `map` to iterate over the result of `to(1, 10)`, `[1,2,3,4,5]`, then
* writes the specified content ("Line $"), which includes the unnamed variable
* `$` for each number in the array.
*
* Note that without `writeLinesWith  "UTF-8"`, the expression
* `{ lines: to(1, 10) map "Line $" }` simply returns
* an array of line numbers as the value of an object:
* `{ "lines": [ "line 1", "line 2", "line 3", "line 4",  "line 5" ] }`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Binaries
* output application/json
* ---
* { lines: to(1, 10) map "Line $" writeLinesWith  "UTF-8" }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "lines": "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n"
* }
* ----
*/
@Since(version = "2.2.0")
fun writeLinesWith(content: Array<String>, charset: String): Binary = native("system::WriteLinesFunctionValue")



/**
* Concatenates the content of two binaries.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `source` | Binary | The source binary content.
* | `with` | Binary | The binary to append.
* |===
*
* === Example
*
* This example concatenates two base-16 values into one binary value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Binaries
* output application/dw
* ---
* "CAFE" as Binary {base: "16"} concatWith "ABCD" as Binary {base: "16"}
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* "yv6rzQ==" as Binary {base: "64"}
* ----
**/
@Since(version = "2.5.0")
@Labels(labels =["concat", "append"])
fun concatWith(source: Binary, with: Binary): Binary = native("system::BinaryAppendBinaryFunctionValue")

/**
* Helper function that enables `concatWith` to work with a `null` value.
*/
fun concatWith(source: Binary, with: Null): Binary = source

/**
* Helper function that enables `concatWith` to work with a `null` value.
*/
fun concatWith(source: Null, with: Binary): Binary = with
