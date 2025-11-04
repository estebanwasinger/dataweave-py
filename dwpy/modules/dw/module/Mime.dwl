/**
* This module contains helper functions for working with MIME type.
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::module::Mime` to the header of your
* DataWeave script.
*/
@Since(version = "2.7.0")
%dw 2.0

/**
 * DataWeave type for representing a MIME type parameter.
 */
@Since(version = "2.7.0")
type MimeTypeParameter = {
  _ ?: String
}

/**
 * DataWeave type for representing a MIME type.
 * Supports the following fields:
 *
 * * `type`: Represents the general category into which the data type falls, such as 'video' or 'text'.
 * * `subtype`: Identifies the exact kind of data of the specified type the MIME type represents.
 * * `parameters`: Parameters attached to the MIME type.
 */
@Since(version = "2.7.0")
type MimeType = {
  'type': String,
  subtype: String,
  parameters: MimeTypeParameter
}

/**
 * DataWeave type of the data that returns when a `fromString` function fails.
 * Supports the following fields:
 *
 * * `message`: The error message.
 */
@Since(version = "2.7.0")
type MimeTypeError = {
  message: String
}

/**
* Transforms a MIME type string value representation to a `MimeType`.
*
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | mimeType | String | The MIME type string value to transform to a `MimeType`.
* |===
*
* === Example
*
* This example transforms a MIME type string value without `parameters` to a `MimeType` value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::module::Mime
* output application/json
* ---
* fromString("application/json")
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "success": true,
*   "result": {
*       "type": "application",
*       "subtype": "json",
*       "parameters": {}
*   }
* }
* ----
*
* === Example
*
* This example transforms a MIME type string value that includes a `parameters` to a `MimeType` value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::module::Mime
* output application/json
* ---
* fromString("multipart/form-data; boundary=the-boundary")
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "success": true,
*   "result": {
*       "type": "multipart",
*       "subtype": "form-data",
*       "parameters": {
*           "boundary": "the-boundary"
*       }
*   }
* }
* ----
*
* === Example
*
* This example transforms an invalid MIME type string value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::module::Mime
* output application/json
* ---
* fromString("Invalid MIME type")
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "success": false,
*   "error": {
*       "message": "Unable to find a sub type in `Invalid MIME type`."
*   }
* }
*
* ----
**/
@Since(version = "2.7.0")
fun fromString(mimeType: String): Result<MimeType, MimeTypeError> = native("system::FromMimeTypeString")

/**
* Transforms a `MimeType` value to a string representation.
*
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | mimeType | `MimeType` | The MIME type value to transform to a `String`.
* |===
*
* === Example
*
* This example transforms a `MimeType` value without `parameters` to a string representation.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::module::Mime
* output application/json
* ---
* toString({'type': "application", subtype: "json", parameters: {}})
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* "application/json"
* ----
*
* === Example
*
* This example transforms a `MimeType` value that includes a `parameters` to a string representation.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::module::Mime
* output application/json
* ---
* toString({'type': "multipart", subtype: "form-data", parameters: {boundary: "my-boundary"}})
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* "multipart/form-data;boundary=my-boundary"
* ----
**/
@Since(version = "2.7.0")
fun toString(mimeType: MimeType): String = native("system::ToMimeTypeString")

/**
* Returns `true` if the given `MimeType` value is handled by the base `MimeType` value.
*
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | base | `MimeType` | The MIME type value used as baseline.
* | other | `MimeType` | The MIME type value to be validated.
* |===
*
* === Example
*
* This example tests how MIME types handles several validations.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::module::Mime
* output application/json
*
* var JSON = {'type': "application", subtype: "json", parameters: {}}
* var MULTIPART = {'type': "multipart", subtype: "form-data", parameters: {boundary: "my-boundary"}}
* var ALL = {'type': "*", subtype: "*", parameters: {}}
* ---
* {
*   a: isHandledBy(JSON, JSON),
*   b: isHandledBy({'type': "*", subtype: "json", parameters: {}}, JSON),
*   c: isHandledBy({'type': "application", subtype: "*", parameters: {}}, JSON),
*   d: isHandledBy(ALL, MULTIPART),
*   e: isHandledBy(MULTIPART, ALL),
*   f: isHandledBy(JSON, MULTIPART),
*   g: isHandledBy(
*     {'type': "application", subtype: "*+xml", parameters: {}},
*     {'type': "application", subtype: "soap+xml", parameters: {}})
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
*   "c": true,
*   "d": true,
*   "e": false,
*   "f": false,
*   "g": true
* }
* ----
*
**/
@Since(version = "2.7.0")
fun isHandledBy(base: MimeType, other: MimeType): Boolean = native("system::IsHandledBy")