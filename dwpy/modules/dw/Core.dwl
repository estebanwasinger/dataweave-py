/**
 * This module contains core DataWeave functions for data transformations.
 * It is automatically imported into any DataWeave script. For documentation
 * on DataWeave _1.0_ functions, see
 * https://docs.mulesoft.com/dataweave/1.2/dataweave-operators[DataWeave Operators].
 */
%dw 2.0

/**
* Annotation that limits the application of an annotation. An example is
* `@AnnotationTarget(targets = ["Function", "Variable"])`, which limits
* the scope of the annotation `annotation TailRec()` to functions and
* variables. If no `AnnotationTarget` is specified, an annotation can
* apply to any valid target.
*
*
* Annotation Targets:
*
* * `Parameter`: For function parameters.
* * `Function`: For function definitions.
* * `Variable`: For variable definitions.
* * `Import`: For import definitions.
*/
annotation AnnotationTarget(targets: Array<"Function" | "Parameter" | "Variable" | "Import" | "Type" | "Version" | "Namespace" | "KeyType" | "TypeExpression" | "Expression" | "Annotation">)

/**
* Annotation that marks a script as untrusted, which means that the script has
* no privileges. For example, such a script cannot gain access to environment
* variables or read a resource from a URL.
*/
@Experimental()
annotation UntrustedCode(privileges: Array<String> = [])

/**
* Annotation that identifies the DataWeave version in which the annotated
* functionality was introduced. An example is `@Since(version = "2.4.0")`.
*
*
* _Introduced in DataWeave 2.3.0. Supported by Mule 4.3 and later._
*/
annotation Since(version: String)

/**
* Annotation that marks another annotation as an Interceptor so that the
* marked annotation will wrap an annotated function with an `interceptorFunction`.
* An example is the `RuntimePrivilege` annotation, which is annotated by
* `@Interceptor(interceptorFunction = "@native system::SecurityManagerCheckFunctionValue")`.
* The `readUrl` function definition is annotated by `@RuntimePrivilege(requires = "Resource")`.
*/
@Experimental()
annotation Interceptor(interceptorFunction: String | (annotationArgs: {}, targetFunctionName: String, args: Array<Any>, callback: (args: Array<Any>) -> Any) -> Any)


/**
* Annotation used to indicate that a function requires runtime privileges to
* execute. An example is `@RuntimePrivilege(requires = "Resource")`, which
* annotates the `readUrl` function definition.
*/
@AnnotationTarget(targets = ["Function"])
@Interceptor(interceptorFunction = "@native system::SecurityManagerCheckFunctionValue")
annotation RuntimePrivilege(requires:String)

/**
* Annotation that marks a parameter type as _design only_ to indicate that
* the field type is validated only at design time. At runtime, only minimal
* type validation takes place. This annotation is useful for performance,
* especially with complex Object types.
*/
@AnnotationTarget(targets = ["Parameter"])
annotation DesignOnlyType()


/**
* Annotation that marks a function as tail recursive. If a function with
* this annotation is not tail recursive, the function will fail.
*/
@AnnotationTarget(targets = ["Function", "Variable"])
annotation TailRec()

/**
* Annotation that marks a function as _internal_ and not to be used.
*
*
*  _Introduced in DataWeave 2.4.0. Supported by Mule 4.4.0 and later._
*/
@Experimental
annotation Internal(permits: Array<String>)

/**
* Annotation that identifies a feature as experimental and subject
* to change or removal in the future.
*/
@Since(version = "2.4.0")
annotation Experimental()

/**
* Annotation that marks a function as deprecated.
*/
@Since(version = "2.4.0")
annotation Deprecated(since:String, replacement: String)

/**
* Annotation for labeling a function or variable definition so that it
* becomes more easy to discover. An example is
* `@Labels(labels =["append", "concat"])`.
*/
@AnnotationTarget(targets = ["Function", "Variable"])
@Since(version = "2.4.0")
annotation Labels(labels: Array<String>)

/**
* Annotation that marks a variable declaration for lazy initialization.
*/
@AnnotationTarget(targets = ["Variable"])
@Since(version = "2.3.0")
annotation Lazy()


/**
* Annotation that marks a parameter as stream capable, which means that this
* field will consume an array of objects in a forward-only manner. Examples of
* functions with `@StreamCapable` fields are `map`, `mapObject`, and `pluck`.
*/
@AnnotationTarget(targets = ["Parameter", "Variable"])
annotation StreamCapable()

/**
* Annotation used to identify the function description to use for the
* function's documentation. This annotation is useful for selecting
* the correct function description when the function is overloaded.
*/
@AnnotationTarget(targets = ["Function"])
@Since(version = "2.4.0")
annotation GlobalDescription()

/**
* Annotation used to mark annotations that represent metadata.
*/
@AnnotationTarget(targets = ["Annotation"])
@Since(version = "2.8.0")
annotation Metadata(key: String)

/**
 * `String` type
 */
type String = ???
/**
* A `Boolean` type of `true` or `false`.
*/
type Boolean = ???
/**
* A number type: Any number, decimal, or integer is represented by the Number` type.
*/
type Number = ???
/**
* A `Range` type represents a sequence of numbers.
*/
type Range = ???
/**
* A `Namespace` type represented by a `URI` and a prefix.
*/
type Namespace = ???
/**
* A URI.
*/
type Uri = ???
/**
* A `Date` and `Time` within a `TimeZone`. For example: `&#124;2018-09-17T22:13:00Z&#124;`
*/
type DateTime = ???
/**
* A `DateTime` in the current `TimeZone`. For example: `&#124;2018-09-17T22:13:00&#124;`
*/
type LocalDateTime = ???
/**
* A date represented by a year, month, and day. For example: `&#124;2018-09-17&#124;`
*/
type Date = ???
/**
* A `Time` in the current `TimeZone`. For example: `&#124;22:10:18&#124;`
*/
type LocalTime = ???
/**
* A time in a specific `TimeZone`. For example: `&#124;22:10:18Z&#124;`
*/
type Time = ???
/**
* A time zone.
*/
type TimeZone = ???
/**
* A period.
*/
type Period = ???
/**
* A blob.
*/
type Binary = ???
/**
* A Null type, which represents the `null` value.
*/
type Null = ???
/**
* A Java regular expression (regex) type.
*/
type Regex = ???
/**
* Bottom type. This type can be assigned to all the types.
*/
type Nothing = ???
/**
 * The top-level type. `Any` extends all of the system types, which
 * means that anything can be assigned to a `Any` typed variable.
 */
type Any = ???
/**
 * Array type that requires a `Type(T)` to represent the elements of the list.
 * Example: `Array<Number>` represents an array of numbers, and `Array<Any>`
 * represents an array of any type.
 *
 * Example: `[1, 2, "a", "b", true, false, { a : "b"}, [1, 2, 3] ]`
 */
type Array<T> = ???
/**
 * Type that represents any object, which is a collection of `Key` and value pairs.
 *
 * Examples: `{ myKey : "a value" }`, `{ myKey : { a : 1, b : 2} }`,
 * `{ myKey : [1,2,3,4] }`
 */

type Object = ???
/**
* A type in the DataWeave type system.
*/
type Type<T> = ???
/**
* A key of an `Object`.
*
* Examples: `{ myKey : "a value" }`, `{ myKey : { a : 1, b : 2} }`,
* `{ myKey : [1,2,3,4] }`
*/
type Key = ???
/**
* Generic dictionary interface.
*/
type Dictionary<T> = {_?: T}
/**
* A union type that represents all the types that can be compared to each other.
*/
type Comparable = String | Number | Boolean | DateTime | LocalDateTime | Date | LocalTime | Time | TimeZone

/**
* A union type that represents all the simple types.
*/
type SimpleType = String | Boolean | Number | DateTime | LocalDateTime | Date | LocalTime | Time | TimeZone | Period

/**
* A union type of all the types that can be coerced to String type.
*/
@Since(version = "2.3.0")
type StringCoerceable = String | Boolean | Number | DateTime | LocalDateTime | Date | LocalTime | Time | TimeZone | Period | Key | Binary | Uri | Type<Any> | Regex | Namespace

/**
* A type used to represent a pair of values.
*/
@Since(version = "2.2.0")
type Pair<LEFT, RIGHT> = {l: LEFT, r: RIGHT}

/**
 * A type for representing an execution result.
 *
 * Supports the following fields:
 *
 * * `success`: Determine if the execution ends successfully. If `true`, the data type provides the `result`. If `false`, the data type provides the `error`.
 * * `result`: The success result data.
 * * `error`: The error data.
 */
@Since(version = "2.7.0")
type Result<T, E> = { success: true, result: T } | { success: false, error: E }

/**
*
* Returns the value of the compatibility flag with the specified name.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `flag` | String | The name of the compatibility flag to evaluate.
* |===
*
* === Example
*
* This example gets the `com.mulesoft.dw.xml_reader.honourMixedContentStructure` compatibility flag value in the current
* DataWeave version.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   "com.mulesoft.dw.xml_reader.honourMixedContentStructure": evaluateCompatibilityFlag("com.mulesoft.dw.xml_reader.honourMixedContentStructure")
* }
*
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "com.mulesoft.dw.xml_reader.honourMixedContentStructure": true
* }
* ----
**/
@Since(version = "2.5.0")
fun evaluateCompatibilityFlag(flag: String): Boolean = native("system::EvaluateCompatibilityFlagFunctionValue")

/**
* Indicates if the compatibility flag 'com.mulesoft.dw.defaultOperator.disableExceptionHandling' is enabled or not.
*/
@Since(version = "2.5.0")
@Internal(permits = ["dw::", "bat::"])
fun isDefaultOperatorDisabledExceptionHandling(): Boolean = evaluateCompatibilityFlag("com.mulesoft.dw.defaultOperator.disableExceptionHandling")

/**
* Indicates if the compatibility flag 'com.mulesoft.dw.legacySizeOfNumber' is enabled or not.
*/
@Since(version = "2.6.0")
@Internal(permits = ["dw::", "bat::"])
fun isLegacySizeOfNumber(): Boolean = evaluateCompatibilityFlag("com.mulesoft.dw.legacySizeOfNumber")

type LogLevel = "Debug" | "Info" | "Warn" | "Error"

/**
* Without changing the value of the input, `logWith` returns the input as a system log at the specified level.
* So this makes it very simple to debug your code, because any expression or subexpression can be wrapped
* with *log* and the result will be printed out without modifying the result of the expression.
* The output is going to be printed in application/dw format.
*
* The prefix parameter is optional and allows to easily find the log output.
*
* The LogLevel is used to categorize log events by severity and control the verbosity of the logs.
*
* Use this function to help with debugging DataWeave scripts. A Mule app
* outputs the results through the `DefaultLoggingService`, which you can see
* in the Studio console.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Description
* | level | Log level that indicates the severity of the event.
* | prefix | An optional string that typically describes the log.
* | value | The value to log.
* |===
**/
@Since(version = "2.10.0")
fun logWith <T>(level: LogLevel, prefix: String, value: T): T = ???

/**
* Helper function that log messages at `Debug` level.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | prefix | An optional string that typically describes the log.
* | value | The value to log.
* |===
*
**/
@Since(version = "2.10.0")
fun logDebug <T>(prefix: String = "", value: T): T = logWith("Debug", prefix, value)

/**
* Helper function that log messages at `Info` level.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | prefix | An optional string that typically describes the log.
* | value | The value to log.
* |===
*
**/
@Since(version = "2.10.0")
fun logInfo <T>(prefix: String = "", value: T): T = logWith("Info", prefix, value)

/**
* Helper function that log messages at `Warn` level.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | prefix | An optional string that typically describes the log.
* | value | The value to log.
* |===
*
**/
@Since(version = "2.10.0")
fun logWarn <T>(prefix: String = "", value: T): T = logWith("Warn", prefix, value)

/**
* Helper function that log messages at `Error` level.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | prefix | An optional string that typically describes the log.
* | value | The value to log.
* |===
*
**/
@Since(version = "2.10.0")
fun logError <T>(prefix: String = "", value: T): T = logWith("Error", prefix, value)

// Internal.
/**
* Internal method used to log message providing the caller context
*/
@Internal(permits = [])
@Since(version = "2.10.0")
fun logInternal <T>(context: Object, level: LogLevel, prefix: String, value: T): T = native("system::log")

/**
* Without changing the value of the input, `log` returns the input as a system
* log. So this makes it very simple to debug your code, because any expression or subexpression can be wrapped
* with *log* and the result will be printed out without modifying the result of the expression.
* The output is going to be printed in application/dw format.
*
*
* The prefix parameter is optional and allows to easily find the log output.
*
*
* Use this function to help with debugging DataWeave scripts. A Mule app
* outputs the results through the `DefaultLoggingService`, which you can see
* in the Studio console.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | prefix | An optional string that typically describes the log.
* | value | The value to log.
* |===
*
* === Example
*
* This example logs the specified message. Note that the `DefaultLoggingService`
* in a Mule app that is running in Studio returns the message
* `WARNING - "Houston, we have a problem,"` adding the dash `-` between the
* prefix and value. The Logger component's `LoggerMessageProcessor` returns
* the input string `"Houston, we have a problem."`, without the `WARNING` prefix.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* log("WARNING", "Houston, we have a problem")
* ----
*
* ==== Output
*
* `Console Output`
*
* [source,XML,linenums]
* ----
* "WARNING - Houston, we have a problem"
* ----
*
* `Expression Output`
*
* [source,XML,linenums]
* ----
* "Houston, we have a problem"
* ----
*
* === Example
*
* This example shows how to log the result of expression `myUser.user` without modifying the
* original expression `myUser.user.friend.name`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
*
* var myUser = {user: {friend: {name: "Shoki"}, id: 1, name: "Tomo"}, accountId: "leansh" }
* ---
* log("User", myUser.user).friend.name
* ----
*
* ==== Output
*
* `Console output`
*
* [source,console,linenums]
* ----
* User - {
*   friend: {
*     name: "Shoki"
*   },
*   id: 1,
*   name: "Tomo"
* }
* ----
*
* `Expression Output`
*
* [source,DataWeave,linenums]
* ----
* "Shoki"
* ----
*/
fun log <T>(prefix: String = "", value: T): T = logWith("Info", prefix, value)

/**
* Reads a string or binary and returns parsed content.
*
*
* This function can be useful if the reader cannot determine the content type
* by default.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | stringToParse | The string or binary to read.
* | contentType | A supported format (or content type). Default: `application/dw`.
* | readerProperties | Optional: Sets reader configuration properties. For other formats and reader configuration properties, see https://docs.mulesoft.com/dataweave/latest/dataweave-formats[Supported Data Formats].
* |===
*
* === Example
*
* This example reads a JSON object `{ "hello" : "world" }'`, and it uses the
* `"application/json"` argument to indicate _input_ content type. By contrast,
* the `output application/xml` directive in the header of the script tells the
* script to transform the JSON content into XML output. Notice that the XML
* output uses `hello` as the root XML element and `world` as the value of
* that element. The `hello` in the XML corresponds to the key `"hello"`
* in the JSON object, and `world` corresponds to the JSON value `"world"`.
*
* ==== Source
*
* [source,dw,linenums]
* ----
* %dw 2.0
* output application/xml
* ---
* read('{ "hello" : "world" }','application/json')
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* <?xml version='1.0' encoding='UTF-8'?><hello>world</hello>
* ----
*
* === Example
*
* This example reads a string as a CSV format without a header and transforms it
* to JSON. Notice that it adds column names as keys to the output object. Also,
* it appends `[0]` to the function call here to select the first index of the
* resulting array, which avoids producing the results within an array (with
* square brackets surrounding the entire output object).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var myVar = "Some, Body"
* output application/json
* ---
* read(myVar,"application/csv",{header:false})[0]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "column_0": "Some", "column_1": " Body" }
* ----
*
* === Example
*
* This example reads the specified XML and shows the syntax for a reader property,
* in this case, `{ indexedReader: "false" }`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/xml
* ---
* {
*    "XML" : read("<prices><basic>9.99</basic></prices>",
*                 "application/xml",
*                 { indexedReader: "false" })."prices"
* }
* ----
*
* ==== Output
*
* [source,XML,linenums]
* ----
* <?xml version='1.0' encoding='UTF-8'?>
* <XML>
*   <basic>9.99</basic>
* </XML>
* ----
*/
fun read(stringToParse: String | Binary, contentType: String = "application/dw", readerProperties: Object = {}): Any = native("system::read")

/**
* Reads a URL, including a classpath-based URL, and returns parsed content.
* This function works similar to the `read` function.
*
*
* The classpath-based URL uses the `classpath:` protocol prefix, for example:
* `classpath://myfolder/myFile.txt` where `myFolder` is located under
* `src/main/resources` in a Mule project. Other than the URL, `readURL` accepts
* the same arguments as `read`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | url | The URL string to read. It also accepts a classpath-based URL.
* | contentType | A supported format (or MIME type). Default: `application/dw`.
* | readerProperties | Optional: Sets reader configuration properties. For other formats and reader configuration properties, see https://docs.mulesoft.com/dataweave/latest/dataweave-formats[Supported Data Formats].
* |===
*
* === Example
*
* This example reads a JSON object from a URL. (For readability, the output
* values shown below are shortened with `...`.)
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* readUrl("https://jsonplaceholder.typicode.com/posts/1", "application/json")
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "userId": 1, "id": 1, "title": "sunt aut ...", "body": "quia et ..." }
* ----
*
* === Example
*
* This example reads a JSON object from a `myJsonSnippet.json` file located in
* the `src/main/resources` directory in Studio. (Sample JSON content for that
* file is shown in the Input section below.) After reading the file contents,
* the script transforms selected fields from JSON to CSV. Reading files
* in this way can be useful when trying out a DataWeave script on sample data,
* especially when the source data is large and your script is complex.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var myJsonSnippet = readUrl("classpath://myJsonSnippet.json", "application/json")
* output application/csv
* ---
* (myJsonSnippet.results map(item) -> item.profile)
* ----
*
* ==== Input
*
* [source,JSON,linenums]
* ----
* {
*   "results": [
*     {
*       "profile": {
*         "firstName": "john",
*         "lastName": "doe",
*         "email": "johndoe@demo.com"
*       },
*       "data": {
*         "interests": [
*           {
*             "language": "English",
*             "tags": [
*               "digital-strategy:Digital Strategy",
*               "innovation:Innovation"
*             ],
*             "contenttypes": []
*           }
*         ]
*       }
*     },
*     {
*       "profile": {
*       "firstName": "jane",
*         "lastName": "doe",
*         "email": "janedoe@demo.com"
*       },
*       "data": {
*         "interests": [
*           {
*             "language": "English",
*             "tags": [
*               "tax-reform:Tax Reform",
*               "retail-health:Retail Health"
*             ],
*             "contenttypes": [
*               "News",
*               "Analysis",
*               "Case studies",
*               "Press releases"
*             ]
*           }
*         ]
*       }
*     }
*   ]
* }
* ----
*
* ==== Output
*
* [source,CSV,linenums]
* ----
* firstName,lastName,email
* john,doe,johndoe@demo.com
* jane,doe,janedoe@demo.com
* ----
*
* === Example
*
* This example reads a CSV file from a URL, sets reader properties to indicate that there's no header, and then transforms the data to JSON.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* readUrl("https://mywebsite.com/data.csv", "application/csv", {"header" : false})
* ----
*
* ==== Input
*
* ----
* Max,the Mule,MuleSoft
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [
*  {
*    "column_0": "Max",
*    "column_1": "the Mule",
*    "column_2": "MuleSoft"
*  }
* ]
* ----
*
* === Example
*
* This example reads a simple `dwl` file from the `src/main/resources`
* directory in Studio, then dynamically reads the value of the key `name`
* from it. (Sample content for the input file is shown in the Input
* section below.)
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* (readUrl("classpath://name.dwl", "application/dw")).firstName
* ----
*
* ==== Input
*
* [source,JSON,linenums]
* ----
* {
*   "firstName" : "Somebody",
*   "lastName" : "Special"
* }
* ----
*
* ==== Output
*
* [source,CSV,linenums]
* ----
* "Somebody"
* ----
*/
@RuntimePrivilege(requires = "Resource")
fun readUrl(url: String, contentType: String = "application/dw", readerProperties: Object = {}): Any = native("system::readUrl")

/**
* Writes a value as a string or binary in a supported format.
*
*
* Returns a String or Binary with the serialized representation of the value
* in the specified format (MIME type). This function can write to a different
* format than the input. Note that the data must validate in that new format,
* or an error will occur. For example, `application/xml` content is not valid
* within an `application/json` format, but `text/plain` can be valid.
* It returns a `String` value for all text-based data formats (such as XML, JSON , CSV)
* and a `Binary` value for all the binary formats (such as Excel, MultiPart, OctetStream).
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | value | The value to write. The value can be of any supported data type.
* | contentType | A supported format (or MIME type) to write. Default: `application/dw`.
* | writerProperties | Optional: Sets writer configuration properties. For writer configuration properties (and other supported MIME types), see https://docs.mulesoft.com/dataweave/latest/dataweave-formats[Supported Data Formats].
* |===
*
* === Example
*
* This example writes the string `world` in plain text (`text/plain"`). It
* outputs that string as the value of a JSON object with the key `hello`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { hello : write("world", "text/plain") }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "hello": "world" }
* ----
*
* === Example
*
* This example takes JSON input and writes the payload to a CSV format that uses a
* pipe (`&#124;`) separator and includes the header (matching keys in the JSON objects).
* Note that if you instead use `"header":false` in your script, the output will
* lack the `Name|Email|Id|Title` header in the output.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/xml
* ---
* { "output" : write(payload, "application/csv", {"header":true, "separator" : "|"}) }
* ----
*
* ==== Input
*
* [source,JSON,linenums]
* ----
* [
*   {
*     "Name": "Mr White",
*     "Email": "white@mulesoft.com",
*     "Id": "1234",
*     "Title": "Chief Java Prophet"
*   },
*   {
*     "Name": "Mr Orange",
*     "Email": "orange@mulesoft.com",
*     "Id": "4567",
*     "Title": "Integration Ninja"
*   }
* ]
* ----
*
* ==== Output
*
* [source,XML,linenums]
* ----
* <?xml version="1.0" encoding="US-ASCII"?>
* <output>Name|Email|Id|Title
* Mr White|white@mulesoft.com|1234|Chief Java Prophet
* Mr Orange|orange@mulesoft.com|4567|Integration Ninja
* </output>
* ----
*/
fun write (value: Any, contentType: String = "application/dw", writerProperties: Object = {}): String | Binary = native("system::write")

/**
* Returns a pseudo-random number greater than or equal to `0.0` and less than `1.0`.
*
* === Example
*
* This example generates a pseudo-random number and multiplies it by 1000.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { price: random() * 1000 }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "price": 65.02770292248383 }
* ----
*/
fun random(): Number = native("system::random")

/**
* Returns a pseudo-random whole number from `0` to the specified number
* (exclusive).
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | upperBound | A number that sets the upper bound of the random number.
* |===
*
* === Example
*
* This example returns an integer from 0 to 1000 (exclusive).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { price: randomInt(1000) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "price": 442.0 }
* ----
*/
fun randomInt(upperBound: Number): Number = floor(random() * upperBound)

/**
* Returns a v4 UUID using random numbers as the source.
*
* === Example
*
* This example generates a random v4 UUID.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* uuid()
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* "7cc64d24-f2ad-4d43-8893-fa24a0789a99"
* ----
*/
fun uuid(): String = native("system::uuid")


/**
* Returns a `DateTime` value for the current date and time.
*
* === Example
*
* This example uses `now()` to return the current date and time as a
* `DateTime` value. It also shows how to return a date and time
* in a specific time zone. Java 8 time zones are supported.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*    nowCalled: now(),
*    nowCalledSpecificTimeZone: now() >> "America/New_York"
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "nowCalled": "2019-08-26T13:32:10.64-07:00",
*   "nowCalledSpecificTimeZone": "2019-08-26T16:32:10.643-04:00"
* }
* ----
*
* === Example
*
* This example shows uses of the `now()` function with valid
* selectors. It also shows how to get the epoch time with `now() as Number`.
* For additional examples, see
* https://docs.mulesoft.com/dataweave/latest/dataweave-types#dw_type_dates[Date and Time (dw::Core Types)].
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   now: now(),
*   epochTime : now() as Number,
*   nanoseconds: now().nanoseconds,
*   milliseconds: now().milliseconds,
*   seconds: now().seconds,
*   minutes: now().minutes,
*   hour: now().hour,
*   day: now().day,
*   month: now().month,
*   year: now().year,
*   quarter: now().quarter,
*   dayOfWeek: now().dayOfWeek,
*   dayOfYear: now().dayOfYear,
*   offsetSeconds: now().offsetSeconds,
*   formattedDate: now() as String {format: "y-MM-dd"},
*   formattedTime: now() as String {format: "hh:m:s"}
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "now": "2019-06-18T16:55:46.678-07:00",
*   "epochTime": 1560902146,
*   "nanoseconds": 678000000,
*   "milliseconds": 678,
*   "seconds": 46,
*   "minutes": 55,
*   "hour": 16,
*   "day": 18,
*   "month": 6,
*   "year": 2019,
*   "quarter": 2,
*   "dayOfWeek": 2,
*   "dayOfYear": 169,
*   "offsetSeconds": -25200,
*   "formattedDate": "2019-06-18",
*   "formattedTime": "04:55:46"
* }
* ----
*/
fun now(): DateTime = native("system::now")

// Internal.
/**
* Internal method used to indicate that a function implementation is not
* written in DataWeave but in Scala.
*/
@Internal(permits = ["dw::", "bat::"])
fun native(identifier: String): Nothing = ??? //This function is just a place holder

// A type: Iterator
/**
* This type is based on the
* https://docs.oracle.com/javase/8/docs/api/java/util/Iterator.html[iterator Java class].
* The iterator contains a collection and includes methods to iterate through
* and filter it.
*
* Just like the Java class, `Iterator` is designed to be consumed only once. For
* example, if you pass it to a
* https://docs.mulesoft.com/dataweave/latest/logger-component-reference[Logger component],
* the Logger consumes it, so it becomes unreadable by further elements in the flow.
**/
type Iterator = Array {iterator: true}

//A type: Enum
/**
* This type is based on the
* https://docs.oracle.com/javase/7/docs/api/java/lang/Enum.html[Enum Java class].
*
* It must always be used with the `class` property, specifying the full Java
* class name of the class, as shown in the example below.
*
* *Source:*
*
* `"Max" as Enum {class: "com.acme.MuleyEnum"}`
*/
type Enum = String {enumeration: true}

// A type: NaN.
/**
* `java.lang.Float` and `java.lang.Double` have special cases for `NaN` and `Infinit`.
* DataWeave does not have these concepts for its number multi-precision nature.
* So when it is mapped to DataWeave values, it is wrapped in a Null with a Schema marker.
*/
type NaN = Null {NaN: true}

// A type: CData.
/**
* XML defines a `CData` custom type that extends from `String` and is used
* to identify a CDATA XML block.
*
* It can be used to tell the writer to wrap the content inside CDATA or to
* check if the string arrives inside a CDATA block. `CData` inherits
* from the type `String`.
*
* *Source*:
*
* `output application/xml --- { "user" : "Shoki" as CData }`
*
* *Output*:
*
* `<?xml version="1.0" encoding="UTF-8"?><user><![CDATA[Shoki]]></user>`
**/
type CData = String {cdata: true}

/**
* Namespace declaration of XMLSchema.
*/
ns xsi http://www.w3.org/2001/XMLSchema-instance

/**
* Creates a `xsi:type` type attribute. This method returns an object, so it must be used with dynamic attributes.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | name | The name of the schema `type` that is referenced without the prefix.
* | namespace | The namespace of that type.
* |===
*
* === Example
*
* This example shows how the `xsiType` behaves under different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/xml
* ns acme http://acme.com
* ---
*   {
*       user @((xsiType("user", acme))): {
*           name: "Peter",
*           lastName: "Parker"
*       }
*   }
* ----
*
* ==== Output
*
* [source,Xml,linenums]
* ----
* <?xml version='1.0' encoding='UTF-8'?>
*  <user xsi:type="acme:user" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:acme="http://acme.com">
*      <name>Peter</name>
*      <lastName>Parker</lastName>
*  </user>
* ----
**/
@Since(version = "2.2.2")
fun xsiType(name: String, namespace: Namespace ) = do {
    var nsObject = namespace as Object
    ---
    {
        xsi#"type": (nsObject.prefix ++   ":" ++ name) as String {nsPrefix: nsObject.prefix, nsUri: nsObject.uri}
    }
}

//---------------------------------------------------------------------------------------------------------

/**
* Concatenates two values.
*
*
* This version of `++` concatenates the elements of two arrays into a
* new array. Other versions act on strings, objects, and the various date and
* time formats that DataWeave supports.
*
* If the two arrays contain different types of elements, the resulting array
* is all of `S` type elements of `Array<S>` followed by all the `T` type elements
* of `Array<T>`. Either of the arrays can also have mixed-type elements. Also
* note that the arrays can contain any supported data type.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | source | The source array.
* | with | The array to concatenate with the source array.
* |===
*
* === Example
*
* The example concatenates an `Array<Number>` with an `Array<String>`. Notice
* that it outputs the result as the value of a JSON object.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "result" : [0, 1, 2] ++ ["a", "b", "c"] }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "result": [0, 1, 2, "a", "b", "c"] }
* ----
*
* === Example
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "a" : [0, 1, true, "my string"] ++ [2, [3,4,5], {"a": 6}] }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a": [0, 1, true, "my string", 2, [3, 4, 5], { "a": 6}] }
* ----
**/
@Labels(labels =["append", "concat"])
fun ++ <S,T>(source: Array<S> , with: Array<T>): Array<S | T> = native("system::ArrayAppendArrayFunctionValue")

/**
* Concatenates the characters of two strings.
*
*
* Strings are treated as arrays of characters, so the `++` operator concatenates
* the characters of each string as if they were arrays of single-character
* string.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | source | The source string.
* | with | The string to concatenate with the source string.
* |===
*
* === Example
*
* This example concatenates two strings. Here, `Mule` is treated as
* `Array<String> ["M", "u", "l", "e"]`. Notice that the example outputs the
* result `MuleSoft` as the value of a JSON object.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "name" : "Mule" ++ "Soft" }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "name": "MuleSoft" }
* ----
**/
@Labels(labels =["append", "concat"])
fun ++(source: String, with: String): String = native("system::StringAppendStringFunctionValue")

/**
* Concatenates two objects and returns one flattened object.
*
*
* The `++` operator extracts all the key-values pairs from each object,
* then combines them together into one result object.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | source | The source object.
* | with | The object to concatenate with the source object.
* |===
*
* === Example
*
* This example concatenates two objects and transforms them to XML. Notice that
* it flattens the array of objects `{aa: "a", bb: "b"}` into separate XML
* elements and that the output uses the keys of the specified JSON objects as
* XML elements and the values of those objects as XML values.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/xml
* ---
* { concat : {aa: "a", bb: "b"} ++ {cc: "c"} }
* ----
*
* ==== Output
*
* [source,XML,linenums]
* ----
* <?xml version="1.0" encoding="UTF-8"?>
* <concat>
*   <aa>a</aa>
*   <bb>b</bb>
*   <cc>c</cc>
* </concat>
* ----
**/
@Labels(labels =["append", "concat"])
fun ++<T <: {} ,Q <: {} >(source: T , with: Q): T & Q = native("system::ObjectAppendObjectFunctionValue")

/**
* Appends a `LocalTime` with a `Date` to return a `LocalDateTime` value.
*
*
* `Date` and `LocalTime` instances are written in standard Java notation,
* surrounded by pipe (`&#124;`) symbols. The result is a `LocalDateTime` object
* in the standard Java format. Note that the order in which the two objects are
* concatenated is irrelevant, so logically, `Date ++ LocalTime` produces the
* same result as `LocalTime ++ Date`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | date | A `Date`.
* | time | A `LocalTime`, a time format without a time zone.
* |===
*
* === Example
*
* This example concatenates a `Date` and `LocalTime` object to return a
* `LocalDateTime`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "LocalDateTime" : (|2017-10-01| ++ |23:57:59|) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "LocalDateTime": "2017-10-01T23:57:59" }
* ----
**/
@Labels(labels =["append", "concat"])
fun ++(date: Date , time: LocalTime): LocalDateTime = native("system::LocalDateAppendLocalTimeFunctionValue")

/**
* Appends a `LocalTime` with a `Date` to return a `LocalDateTime`.
*
*
* Note that the order in which the two objects are concatenated is irrelevant,
* so logically, `LocalTime ++ Date` produces the same result as
* `Date ++ LocalTime`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | time | A `LocalTime`, a time format without a time zone.
* | date | A `Date`.
* |===
*
* === Example
*
* This example concatenates `LocalTime` and `Date` objects to return a
* `LocalDateTime`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "LocalDateTime" : (|23:57:59| ++ |2003-10-01|) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "LocalDateTime": "2017-10-01T23:57:59" }
* ----
**/
@Labels(labels =["append", "concat"])
fun ++(time: LocalTime , date: Date): LocalDateTime = native("system::LocalTimeAppendLocalDateFunctionValue")

/**
* Appends a `Date` to a `Time` in order to return a `DateTime`.
*
*
* Note that the order in which the two objects are concatenated is irrelevant,
* so logically, `Date` + `Time`  produces the same result as `Time` + `Date`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | date | A `Date`.
* | time | A `Time`, a time format that can include a time zone (`Z` or `HH:mm`).
* |===
*
* === Example
*
* This example concatenates `Date` and `Time` objects to return a `DateTime`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ |2017-10-01| ++ |23:57:59-03:00|, |2017-10-01| ++ |23:57:59Z| ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ "2017-10-01T23:57:59-03:00", "2017-10-01T23:57:59Z" ]
* ----
**/
@Labels(labels =["append", "concat"])
fun ++(date: Date , time: Time): DateTime = native("system::LocalDateAppendTimeFunctionValue")

/**
* Appends a `Date` to a `Time` object to return a `DateTime`.
*
*
* Note that the order in which the two objects are concatenated is irrelevant,
* so logically, `Date` + `Time`  produces the same result as a `Time` + `Date`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | time | A `Time`, a time format that can include a time zone (`Z` or `HH:mm`).
* | date | A `Date`.
* |===
*
* === Example
*
* This example concatenates a `Date` with a `Time` to output a `DateTime`.
* Notice that the inputs are surrounded by pipes (`&#124;`).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* |2018-11-30| ++ |23:57:59+01:00|
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* "2018-11-30T23:57:59+01:00"
* ----
*
* === Example
*
* This example concatenates `Time` and `Date` objects to return `DateTime`
* objects. Note that the first `LocalTime` `object is coerced to a `Time`.
* Notice that the order of the date and time inputs does not change the order
* of the output `DateTime`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   "DateTime1" : (|23:57:59| as Time) ++ |2017-10-01|,
*   "DateTime2" : |23:57:59Z| ++ |2017-10-01|,
*   "DateTime3" : |2017-10-01| ++ |23:57:59+02:00|
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "DateTime1": "2017-10-01T23:57:59Z",
*   "DateTime2": "2017-10-01T23:57:59Z",
*   "DateTime3": "2017-10-01T23:57:59+02:00"
* }
* ----
**/
@Labels(labels =["append", "concat"])
fun ++(time: Time , date: Date): DateTime = native("system::TimeAppendLocalDateFunctionValue")

/**
* Appends a `TimeZone` to a `Date` type value and returns a `DateTime` result.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | date | A `Date`.
* | timezone | A `TimeZone` (`Z` or `HH:mm`).
* |===
*
* === Example
*
* This example concatenates `Date` and `TimeZone` (`-03:00`) to return a
* `DateTime`. Note the local time in the `DateTime` is `00:00:00` (midnight).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "DateTime" : (|2017-10-01| ++ |-03:00|) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "DateTime": "2017-10-01T00:00:00-03:00" }
* ----
*
**/
@Labels(labels =["append", "concat"])
fun ++(date: Date , timezone: TimeZone): DateTime = native("system::LocalDateAppendTimeZoneFunctionValue")

/**
* Appends a `Date` to a `TimeZone` in order to return a `DateTime`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | date | A `Date`.
* | timezone | A `TimeZone` (`Z` or `HH:mm`).
* |===
*
* === Example
*
* This example concatenates `TimeZone` (`-03:00`) and `Date` to return a
* `DateTime`. Note the local time in the `DateTime` is `00:00:00` (midnight).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "DateTime" : |-03:00| ++ |2017-10-01| }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "DateTime": "2017-10-01T00:00:00-03:00" }
* ----
*
**/
@Labels(labels =["append", "concat"])
fun ++(timezone: TimeZone , date: Date): DateTime = native("system::TimeZoneAppendLocalDateFunctionValue")

/**
* Appends a `TimeZone` to a `LocalDateTime` in order to return a `DateTime`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | dateTime | A `LocalDateTime`, a date and time without a time zone.
* | timezone | A `TimeZone` (`Z` or `HH:mm`).
* |===
*
* === Example
*
* This example concatenates `LocalDateTime` and `TimeZone` (`-03:00`) to return a
* `DateTime`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "DateTime" : (|2003-10-01T23:57:59| ++ |-03:00|) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "DateTime": "2003-10-01T23:57:59-03:00 }
* ----
**/
@Labels(labels =["append", "concat"])
fun ++(dateTime: LocalDateTime , timezone: TimeZone): DateTime = native("system::LocalDateTimeAppendTimeZoneFunctionValue")

/**
* Appends a `LocalDateTime` to a `TimeZone` in order to return a `DateTime`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | dateTime | A `LocalDateTime`, a date and time without a time zone.
* | timezone | A `TimeZone` (`Z` or `HH:mm`).
* |===
*
* === Example
*
* This example concatenates `TimeZone` (`-03:00`) and `LocalDateTime` to return
* a `DateTime`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "TimeZone" : (|-03:00| ++ |2003-10-01T23:57:59|) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "TimeZone": "2003-10-01T23:57:59-03:00" }
* ----
**/
@Labels(labels =["append", "concat"])
fun ++(timezone: TimeZone , datetime: LocalDateTime): DateTime = native("system::TimeZoneAppendLocalDateTimeFunctionValue")

/**
* Appends a `TimeZone` to a `LocalTime` in order to return a `Time`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | time | A `LocalTime`, time format without a time zone.
* | timezone | A `TimeZone` (`Z` or `HH:mm`).
* |===
*
* === Example
*
* This example concatenates `LocalTime` and `TimeZone` (`-03:00`) to return a
* `Time`. Note that the output returns`:00` for the unspecified seconds.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "Time" : (|23:57| ++ |-03:00|) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "Time": "23:57:00-03:00" }
* ----
*
**/
@Labels(labels =["append", "concat"])
fun ++ (time: LocalTime, timezone: TimeZone): Time = native('system::LocalTimeAppendTimeZoneFunctionValue')

/**
* Appends a `LocalTime` to a `TimeZone` in order to return a `Time`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | time | A `LocalTime`, a time format without a time zone.
* | timezone | A `TimeZone` (`Z` or `HH:mm`).
* |===
*
* === Example
*
* This example concatenates `TimeZone` (`-03:00`) and `LocalTime` to return a
* `Time`. Note that the output returns`:00` for the unspecified seconds.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "Time" : (|-03:00| ++ |23:57|) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "Time": "23:57:00-03:00"
* }
* ----
**/
@Labels(labels =["append", "concat"])
fun ++ (timezone: TimeZone, time: LocalTime): Time = native('system::TimeZoneValueAppendLocalTimeFunctionValue')

/**
* Iterates over an object using a mapper that acts on keys, values, or
* indices of that object.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | object | The object to map.
* | mapper | Expression or selector that provides the `key`, `value`, or `index` used for mapping the specified object into an output object.
* |===
*
* === Example
*
* This example iterates over the input `{ "a":"b","c":"d"}` and uses the
* anonymous mapper function (`(value,key,index) -> { (index) : { (value):key} }`)
* to invert the keys and values in each specified object and to return the
* indices of the objects as keys. The mapper uses named parameters to identify
* the keys, values, and indices of the input object. Note that you can write
* the same expression using anonymous parameters, like this:
* `{"a":"b","c":"d"} mapObject { (&#36;&#36;&#36;) : { (&#36;):&#36;&#36;} }`
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {"a":"b","c":"d"} mapObject (value,key,index) -> { (index) : { (value):key} }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "0": { "b": "a" }, "1": { "d": "c" } }
* ----
*
* === Example
*
* This example increases each price by 5 and formats the numbers to always
* include 2 decimals.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/xml
* ---
* {
*     prices: payload.prices mapObject (value, key) -> {
*         (key): (value + 5) as Number {format: "##.00"}
*     }
* }
* ----
*
* ==== Input
*
* [source,XML,linenums]
* ----
* <?xml version='1.0' encoding='UTF-8'?>
* <prices>
*     <basic>9.99</basic>
*     <premium>53</premium>
*     <vip>398.99</vip>
* </prices>
* ----
*
* ==== Output
*
* [source,XML,linenums]
* ----
* <?xml version='1.0' encoding='UTF-8'?>
* <prices>
*   <basic>14.99</basic>
*   <premium>58.00</premium>
*   <vip>403.99</vip>
* </prices>
* ----
**/
fun mapObject <K,V>(@StreamCapable object: {(K)?: V}, mapper : (value: V, key: K, index: Number) -> Object): Object = native('system::MapObjectObjectFunctionValue')

/**
* Helper function that enables `mapObject` to work with a `null` value.
*
* === Example
*
* Using the previous example, you can test that if the input of the `mapObject`
* is `null`, the output result is `null` as well. In XML `null` values are
* written as empty tags. You can change these values by using the writer
* property `writeNilOnNull=true`.
*
* ==== Input
*
* [source,XML,linenums]
* ----
* <?xml version='1.0' encoding='UTF-8'?>
* <prices>
* </prices>
* ----
*
* ==== Output
*
* [source,XML,linenums]
* ----
* <?xml version='1.0' encoding='UTF-8'?>
* <prices>
* </prices>
* ----
*/
fun mapObject(value: Null, mapper : (value: Nothing, key: Nothing, index: Nothing) -> Any): Null = null

/**
* Useful for mapping an object into an array, `pluck` iterates over an object
* and returns an array of keys, values, or indices from the object.
*
*
* It is an alternative to `mapObject`, which is similar but returns
* an object, instead of an array.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | object | The object to map.
* | mapper | Expression or selector that provides the `key`, `value`, and/or `index` (optional) used for mapping the specified object into an array.
* |===
*
* === Example
*
* This example iterates over `{ "a":"b","c":"d"}` using the
* anonymous mapper function (`(value,key,index) -> { (index) : { (value):key} }`)
* to invert each key-value pair in the specified object and to return their
* indices as keys. The mapper uses named parameters to identify
* the keys, values, and indices of the object. Note that you can write
* the same expression using anonymous parameters, like this:
* `{"a":"b","c":"d"} pluck { (&#36;&#36;&#36;) : { (&#36;):&#36;&#36;} }`
* Unlike the almost identical example that uses `mapObject`, `pluck` returns
* the output as an array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {"a":"b","c":"d"} pluck (value,key,index) -> { (index) : { (value):key} }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ { "0": { "b": "a" } }, { "1": { "d": "c" } } ]
* ----
*
* === Example
*
* This example uses `pluck` to iterate over each element within `<prices/>`
* and returns arrays of their keys, values, and indices. It uses anonymous
* parameters to capture them. Note that it uses `as Number` to convert the
* values to numbers. Otherwise, they would return as strings.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* var readXml = read("<prices>
*     <basic>9.99</basic>
*     <premium>53.00</premium>
*     <vip>398.99</vip>
*     </prices>", "application/xml")
* ---
* "result" : {
*   "keys" : readXml.prices pluck($$),
*   "values" : readXml.prices pluck($) as Number,
*   "indices" : readXml.prices pluck($$$)
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*    "result": {
*      "keys": [ "basic", "premium", "vip" ],
*      "values": [ 9.99, 53, 398.99 ],
*      "indices": [ 0, 1, 2 ]
*    }
* }
* ----
*/
fun pluck <K,V,R>(@StreamCapable object: {(K)?: V}, mapper: (value: V,  key: K, index: Number) -> R): Array<R> = native('system::PluckObjectFunctionValue')

/**
* Helper function that enables `pluck` to work with a `null` value.
*/
fun pluck(value: Null, mapper:(value: Nothing, key: Nothing, index: Nothing) -> Any): Null = null

/**
* Merges elements from two arrays into an array of arrays.
*
*
* The first sub-array in the output array contains the first indices of the input
* sub-arrays. The second index contains the second indices of the inputs, the third
* contains the third indices, and so on for every case where there are the same
* number of indices in the arrays.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | left | The array on the left-hand side of the function.
* | right | The array on the right-hand side of the function.
* |===
*
* === Example
*
* This example zips the arrays located to the left and right of `zip`. Notice
* that it returns an array of arrays where the first index, (`[0,1]`) contains
* the first indices of the specified arrays. The second index of the output array
* (`[1,"b"]`) contains the second indices of the specified arrays.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [0,1] zip ["a","b"]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ [0,"a"], [1,"b"] ]
* ----
*
* === Example
*
* This example zips elements of the left-hand and right-hand arrays. Notice
* that only elements with counterparts at the same index are returned in the
* array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   "a" : [0, 1, 2, 3] zip ["a", "b", "c", "d"],
*   "b" : [0, 1, 2, 3] zip ["a"],
*   "c" : [0, 1, 2, 3] zip ["a", "b"],
*   "d" : [0, 1, 2] zip ["a", "b", "c", "d"]
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "a": [
*     [0,"a"],
*     [1,"b"],
*     [2,"c"],
*     [3,"d"]
*     ],
*   "b": [
*     [0,"a"]
*   ],
*   "c": [
*     [0,"a"],
*     [1,"b"]
*   ],
*   "d": [
*     [0,"a"],
*     [1,"b"],
*     [2,"c"]
*   ]
* }
* ----
*
* === Example
*
* This example zips more than two arrays. Notice that items from
* `["aA", "bB"]` in `list4` are not in the output because the other input
* arrays only have two indices.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* var myvar = {
*    "list1": ["a", "b"],
*    "list2": [1, 2, 3],
*    "list3": ["aa", "bb"],
*    "list4": [["A", "B", "C"], [11, 12], ["aA", "bB"]]
* }
* ---
* ((myvar.list1 zip myvar.list2) zip myvar.list3) zip myvar.list4
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [
*   [
*     [ [ "a", 1 ], "aa" ], [ "A", "B", "C" ]
*   ],
*   [
*     [ [ "b", 2 ], "bb" ], [ 11, 12 ]
*   ]
* ]
* ----
**/
fun zip<T,R>(left: Array<T>, right: Array<R>): Array<Array<T | R>> =
   left match {
       case [lh ~ ltail] ->
         right match {
            case [rh ~ rtail] -> [[lh, rh] ~ zip(ltail, rtail)]
            case [] -> []
         }
       case [] -> []
   }

/**
* Iterates over items in an array and outputs the results into a new array.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | items | The array to map.
* | mapper | Expression or selector used to act on each `item` and optionally,  each `index` of that item.
* |===
*
* === Example
*
* This example iterates over an input array (`["jose", "pedro", "mateo"]`) to
* produce an array of DataWeave objects. The anonymous function
* `(value, index) -> {index: value}` maps each item in the input to an object.
* As `{index: value}` shows, each index from the input array becomes a key
* for an output object, and each value of the input array becomes the value of
* that object.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* ["jose", "pedro", "mateo"] map (value, index) -> { (index) : value}
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ { "0": "jose" }, { "1": "pedro" }, { "2": "mateo" } ]
* ----
*
* === Example
*
* This example iterates over the input array (`['a', 'b', 'c']`) using
* an anonymous function that acts on the items and indices of the input. For
* each item in the input array, it concatenates the `index + 1` (`index` plus 1)
* with an underscore (`_`), and the corresponding `value` to return the array,
* `[ "1_a", "2_b", "3_c" ]`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* ['a', 'b', 'c'] map ((value, index) -> (index + 1) ++ '_' ++ value)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ "1_a", "2_b", "3_c" ]
* ----
*
* === Example
*
* If the parameters of the `mapper` function are not named, the index can be
* referenced with `&#36;&#36;`, and the value with `&#36;`. This example
* iterates over each item in the input array `['joe', 'pete', 'matt']`
* and returns an array of objects where the index is selected as the key.
* The value of each item in the array is selected as the value of
* the returned object. Note that the quotes around `&#36;&#36;`
* are necessary to convert the numeric keys to strings.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* ['joe', 'pete', 'matt'] map ( "$$" : $)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [
*   { "0": "joe" },
*   { "1": "pete" },
*   { "2": "matt" }
* ]
* ----
*
* === Example
*
* This example iterates over a list of objects and transform the values into CSV. Each of these objects represent a CSV row. The `map` operation generates an object with `age` and `address` for each entry in the list. `$` represents the implicit variable under iteration.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/csv
* ---
* [{
*  "age": 14 ,
*  "name": "Claire"
* }, {
*  "age": 56,
*  "name": "Max"
* }, {
*  "age": 89,
*  "name": "John"
* }] map {
*    age: $.age,
*    name: $.name
* }
* ----
*
* ==== Output
*
* [source,CSV,linenums]
* ----
* age,name
* 14,Claire
* 56,Max
* 89,John
* ----
**/
@Labels(labels = ["foreach", "transform"])
fun map <T,R>(@StreamCapable items: Array<T>, mapper: (item: T, index: Number) -> R ): Array<R> = native("system::ArrayMapFunctionValue")

/**
* Helper function that enables `map` to work with a `null` value.
*/
@Labels(labels = ["foreach", "transform"])
fun map(@StreamCapable value: Null, mapper: (item: Nothing, index: Nothing) -> Any): Null = null

/**
* Iterates over each item in an array and flattens the results.
*
*
* Instead of returning an array of arrays (as `map` does when you iterate over
* the values within an input like `[ [1,2], [3,4] ]`), `flatMap` returns a
* flattened array that looks like this: `[1, 2, 3, 4]`. `flatMap` is similar to
* `flatten`, but `flatten` only acts on the values of the arrays, while
* `flatMap` can act on values and indices of items in the array.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | items | The array to map.
* | mapper | Expression or selector for an `item` and/or `index` in the array to flatten.
* |===
*
* === Example
*
* This example returns an array containing each value in order. Though it names
* the optional `index` parameter in its anonymous function
* `(value, index) -> value`, it does not use `index` as a selector for the
* output, so it is possible to write the anonymous function using
* `(value) -> value`. You can also use an anonymous parameter for the
* value to write the example like this: `[ [3,5], [0.9,5.5] ] flatMap &#36;`.
* Note that this example produces the same result as
* `flatten([ [3,5], [0.9,5.5] ])`, which uses `flatten`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ [3,5], [0.9,5.5] ] flatMap (value, index) -> value
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ 3, 5, 0.9, 5.5]
* ----
*/
fun flatMap<T,R>(@StreamCapable items: Array<T>, mapper:(item: T, index: Number) -> Array<R>): Array<R> =
    flatten(items map (value,index) -> mapper(value,index))

/**
* Helper function that enables `flatMap` to work with a `null` value.
*/
fun flatMap<T,R>(@StreamCapable value: Null, mapper: (item: Nothing, index: Nothing) -> Any): Null = null

/**
* Iterates over an array and applies an expression that returns matching values.
*
*
* The expression must return `true` or `false`. If the expression returns `true`
* for a value or index in the array, the value gets captured in the output array.
* If it returns `false` for a value or index in the array, that item gets
* filtered out of the output. If there are no matches, the output array will
* be empty.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | items | The array to filter.
* | criteria | Boolean expression that selects an `item` and/or `index`.
* |===
*
* === Example
*
* This example returns an array of values in the array that are greater than `2`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* [9,2,3,4,5] filter (value, index) -> (value > 2)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [9,3,4,5]
* ----
*
* === Example
*
* This example returns an array of all the users with age bigger or equal to 30.
* The script accesses data of each element from within the lambda expression.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* ---
* [{name: "Mariano", age: 37}, {name: "Shoki", age: 30}, {name: "Tomo", age: 25}, {name: "Ana", age: 29}]
*           filter ((value, index) -> value.age >= 30)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [
*    {
*      "name": "Mariano",
*      "age": 37
*    },
*    {
*      "name": "Shoki",
*      "age": 30
*    }
* ]
* ----
*
* === Example
*
* This example returns an array of all items found at an index (`&#36;&#36;`)
* greater than `1` where the value of the element is less than `5`. Notice that
* it is using anonymous parameters as selectors instead of using named
* parameters in an anonymous function.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [9, 2, 3, 4, 5] filter (($$ > 1) and ($ < 5))
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [3,4]
* ----
*
* === Example
*
* This example reads a JSON array that contains objects with `user` and `error` keys, and uses the `filter` function to return only the objects in which the value of the `error` key is `null`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
*
* var users = [
*    {
*       "user": {
*          "name": "123",
*          "lastName": "Smith"
*       },
*       "error": "That name doesn't exists"
*    },
*    {
*       "user": {
*          "name": "John",
*          "lastName": "Johnson"
*       },
*       "error": null
*    }
* ]
* ---
* users filter ((item, index) -> item.error == null)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [
*   {
*     "user": {
*       "name": "John",
*       "lastName": "Johnson"
*     },
*     "error": null
*   }
* ]
* ----
* 
* === Example
*
* This example reads a JSON array and uses the `filter` function to extract the phone numbers that are active.
*
* ==== Input
*
* [source,JSON, linenums]
* ----
* {
*   "Id": "1184001100000000517",
*   "marketCode": "US",
*   "languageCode": "en-US",
*   "profile": {
*   "base": {
*      "username": "TheMule",
*      "activeInd": "R",
*      "phone": [
*        {
*           "activeInd": "Y",
*           "type": "mobile",
*           "primaryInd": "Y",
*           "number": "230678123"
*        },
*        {
*          "activeInd": "N",
*          "type": "mobile",
*          "primaryInd": "N",
*          "number": ""
*        },
*        {
*           "activeInd": "Y",
*           "type": "mobile",
*           "primaryInd": "Y",
*           "number": "154896523"
*        }
*       ]
*     }
*   }
*  }
* ----
*
* ==== Source
* [source,DataWeave, linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*     id: payload.Id,
*     markCode: payload.marketCode,
*     languageCode: payload.languageCode,
*     username: payload.profile.base.username,
*     phoneNumber: (payload.profile.base.phone filter ($.activeInd == "Y" and $.primaryInd== "Y")).number default []
* }
* ----
*
* ==== Output
* [source,JSON, linenums]
* ----
* {
*   "id": "1184001100000000517",
*   "markCode": "US",
*   "languageCode": "en-US",
*   "username": "TheMule",
*   "phoneNumber": [
*     "230678123"
*     "154896523"
*   ]
* }
* ----
*
**/
@Labels(labels = ["where"])
fun filter <T>(@StreamCapable items: Array<T> , criteria: (item: T, index: Number) -> Boolean): Array<T> = native("system::ArrayFilterFunctionValue")

/**
*
* Iterates over a string and applies an expression that returns matching values.
*
*
* The expression must return `true` or `false`. If the expression returns `true`
* for a character or index in the array, the character gets captured in the output string.
* If it returns `false` for a character or index in the array, that character gets
* filtered out of the output. If there are no matches, the output string will
* be empty.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The text to filter.
* | criteria | The criteria to use.
* |===
*
* === Example
*
* This example shows how `filter` can be used to remove all characters in odd positions.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* "hello world" filter ($$ mod 2) == 0
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* "hlowrd"
* ----
**/
fun filter(@StreamCapable text: String, criteria: (character: String, index: Number) -> Boolean): String =
    (text reduce ((item, acc={idx:0, result: ""}) ->
    if (criteria(item, acc.idx))
        {idx: acc.idx + 1, result: acc.result ++ item}
    else
        {idx: acc.idx + 1, result: acc.result}
    )).result

/**
* Helper function that enables `filter` to work with a `null` value.
*/
@Labels(labels = ["where"])
fun filter(@StreamCapable value: Null, criteria: (item: Nothing, index: Nothing) -> Any): Null = null

/**
* Iterates a list of key-value pairs in an object and applies an expression that
* returns only matching objects, filtering out the rest from the output.
*
*
* The expression must return `true` or `false`. If the expression returns `true`
* for a key, value, or index of an object, the object gets captured in the
* output. If it returns `false` for any of them, the object gets filtered out
* of the output. If there are no matches, the output array will be empty.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | The source object to evaluate.
* | criteria | Boolean expression that selects a `value`, `key`, or `index` of the object.
* |===
*
* === Example
*
* This example outputs an object if its value equals `"apple"`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {"a" : "apple", "b" : "banana"} filterObject ((value) -> value == "apple")
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a": "apple" }
* ----
*
* === Example
*
* This example only outputs an object if the key starts with "letter". The
* DataWeave `startsWith` function returns `true` or `false`. Note that you can
* use the anonymous parameter for the key to write the expression
* `((value, key) -> key startsWith "letter")`: (&#36;&#36; startsWith "letter")`
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {"letter1": "a", "letter2": "b", "id": 1} filterObject ((value, key) -> key startsWith "letter")
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "letter1": "a", "letter2": "b" }
* ----
*
* === Example
*
* This example only outputs an object if the index of the object in the array
* is less than 1, which is always true of the first object. Note that you can
* use the anonymous parameter for the index to write the expression
* `((value, key, index) -> index < 1)`: `(&#36;&#36;&#36; < 1)`
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "1": "a", "2": "b", "3": "c"} filterObject ((value, key, index) -> index < 1)
* ----
*
* ==== Output
*
* [source,json,linenums]
* ----
* { "1": "a" }
* ----
*
* === Example
*
* This example outputs an object that contains only the values that are not `null` in the input JSON object.
*
* ==== Source
*
* [source,DataWeave, linenums]
* ----
* %dw 2.0
* output application/json
* var myObject = {
*     str1 : "String 1",
*     str2 : "String 2",
*     str3 : null,
*     str4 : "String 4",
* }
* ---
* myObject filterObject $ != null
* ----
*
* ==== Output
*
* [source,json,linenums]
* ----
* {
*   "str1": "String 1",
*   "str2": "String 2",
*   "str4": "String 4"
* }
* ----
*/
fun filterObject <K,V>(@StreamCapable value: {(K)?: V}, criteria: (value: V, key: K, index: Number) -> Boolean): {(K)?: V} = native("system::ObjectFilterFunctionValue")

/**
* Helper function that enables `filterObject` to work with a `null` value.
*/
fun filterObject(value: Null, criteria: (value: Nothing, key: Nothing, index: Nothing) -> Any): Null = null

/**
 * Returns an array of key-value pairs that describe the key, value, and any
 * attributes in the input object.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | obj | The object to describe.
 * |===
 *
 * === Example
 *
 * This example returns the key, value, and attributes from the object specified
 * in the variable `myVar`. The object is the XML input to the `read` function.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * var myVar = read('<xml attr="x"><a>true</a><b>1</b></xml>', 'application/xml')
 * output application/json
 * ---
 * { "entriesOf" : entriesOf(myVar) }
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * {
 *   "entriesOf": [
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
@Since(version = "2.3.0")
fun entriesOf<T <: Object>(obj: T): Array<{|key: Key, value: Any, attributes: Object|}> =
  obj pluck (value, key) -> {
    key: key,
    value: value,
    attributes: key.@ default {}
  }

/**
* Helper function that enables `entriesOf` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun entriesOf(obj: Null): Null = null

/**
* Returns an array of strings with the names of all the keys within the given object.
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
* This example returns the keys from the key-value pairs within the input object.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "namesOf" : namesOf({ "a" : true, "b" : 1}) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "namesOf" : ["a","b"] }
* ----
*/
@Since(version = "2.3.0")
fun namesOf(obj: Object): Array<String> = obj pluck ($$ as String)

/**
* Helper function that enables `namesOf` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun namesOf(obj: Null): Null = null

/**
* Returns an array of keys from key-value pairs within the input object.
*
*
* The returned keys belong to the Key type. To return each key as a string, you can use `namesOf`, instead.
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
* This example returns the keys from the key-value pairs within the input object.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "keysOf" : keysOf({ "a" : true, "b" : 1}) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "keysOf" : ["a","b"] }
* ----
*
* === Example
*
* This example illustrates a difference between `keysOf` and `namesOf`.
* Notice that `keysOf` retains the attributes (`name` and `lastName`)
* and namespaces (`xmlns`) from the XML input, while `namesOf` returns
* `null` for them because it does not retain them.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var myVar = read('<users xmlns="http://test.com">
*                      <user name="Mariano" lastName="Achaval"/>
*                      <user name="Stacey" lastName="Duke"/>
*                   </users>', 'application/xml')
* output application/json
* ---
* { keysOfExample: flatten([keysOf(myVar.users) map $.#,
*                           keysOf(myVar.users) map $.@])
* }
* ++
* { namesOfExample: flatten([namesOf(myVar.users) map $.#,
*                     namesOf(myVar.users) map $.@])
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "keysOfExample": [
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
*   "namesOfExample": [
*     null,
*     null,
*     null,
*     null
*   ]
* }
* ----
*/
@Since(version = "2.3.0")
fun keysOf<K,V>(obj: {(K)?: V}): Array<K> = obj pluck $$

/**
* Helper function that enables `keysOf` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun keysOf(obj: Null): Null = null

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
* This example returns the values of key-value pairs within the input object.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "valuesOf" : valuesOf({a: true, b: 1}) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "valuesOf" : [true,1] }
* ----
*/
@Since(version = "2.3.0")
fun valuesOf <K,V>(obj: {(K)?: V}): Array<V> = obj pluck $

/**
* Helper function that enables `valuesOf` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun valuesOf (obj: Null): Null = null

/**
* Performs string replacement.
*
*
* This version  of `replace` accepts a Java regular expression for matching
* part of a string. It requires the use of the `with` helper function to
* specify a replacement string for the matching part of the input string.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | A string to match.
* | matcher | A Java regular expression for matching characters in the input `text` string.
* |===
*
* === Example
*
* The first example in the source replaces all characters up to and including
* the second hyphen (`123-456-`) with an empty value, so it returns the last
* four digits. The second replaces the characters `b13e` in the input string
* with a hyphen (`-`).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* ["123-456-7890" replace /.*-/ with(""), "abc123def" replace /[b13e]/ with("-")]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ 7890, "a-c-2-d-f" ]
* ----
*
* === Example
*
* This example replaces the numbers `123` in the input strings with `ID`. It
* uses the regular expression `(\d+)`, where the `\d` metacharacter means any
* digit from 0-9, and `+` means that the digit can occur one or more times.
* Without the `+`, the output would contain one `ID` per digit. The example
* also shows how to write the expression using infix notation, then using
* prefix notation.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ "my123" replace /(\d+)/ with("ID"), replace("myOther123", /(\d+)/) with("ID") ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ "myID", "myOtherID" ]
* ----
**/
fun replace(text: String, matcher: Regex): ((Array<String>, Number) -> String) -> String = native("system::ReplaceStringRegexFunctionValue")

/**
* Performs string replacement.
*
*
* This version of `replace` accepts a string that matches part of a specified
* string. It requires the use of the `with` helper function to pass in a
* replacement string for the matching part of the input string.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The string to match.
* | matcher | The string for matching characters in the input `text` string.
* |===
*
* === Example
*
* This example replaces the numbers `123` from the input string with
* the characters `ID`, which are passed through the `with` function.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "replace": "admin123" replace "123" with("ID") }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "replace": "adminID" }
* ----
**/
fun replace(text: String, matcher: String): ((Array<String>, Number) -> String) -> String = native("system::ReplaceStringStringFunctionValue")

/**
* Helper function that enables `replace` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun replace(text: Null, matcher: Any): ((Nothing, Nothing) -> Any) -> Null = (f) -> null

/**
* Helper function that specifies a replacement element. This function is used with `replace`, `update` or `mask` to perform data substitutions.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | toBeReplaced | The value to be replaced.
* | replacer | The replacement value for the input value.
* |===
*
* === Example
*
* This example replaces all numbers in a string with "x" characters. The `replace` function specifies the base string and a regex to select the characters to replace, and `with` provides the replacement string to use.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "ssn" : "987-65-4321" replace /[0-9]/ with("x") }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "ssn": "xxx-xx-xxxx" }
* ----
*/
fun with<V,U,R,X>(toBeReplaced: ((V, U) -> R) -> X, replacer: (V, U) -> R ): X = toBeReplaced(replacer)


/**
* Applies a reduction expression to the elements in an array.
*
*
* For each element of the input array, in order, `reduce` applies the reduction
* lambda expression (function), then replaces the accumulator with the new
* result. The lambda expression can use both the current input array element
* and the current accumulator value.
*
* Note that if the array is empty and no default value is set on the
* accumulator parameter, a null value is returned.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | item | Item in the input array. It provides the value to reduce. Can also be referenced as `&#36;`.
* | acc | The accumulator. Can also be referenced as `&#36;&#36;`. Used to store the result of the lambda expression after each iteration of the `reduce` operation.
*
* The accumulator parameter can be set to an initial value using the
* syntax `acc = initValue`. In this case, the lambda expression is
* called with the first element of the input array. Then the result
* is set as the new accumulator value.
*
* If an initial value for the accumulator is not set, the accumulator
* is set to the first element of the input array. Then the lambda
* expression is called with the second element of the input array.
*
* The initial value of the accumulator and the lambda expression
* dictate the type of result produced by the `reduce` function. If
* the accumulator is set to `acc = {}`, the result is usually of type
* `Object`. If the accumulator is set to `acc = []`, the result
* is usually of type `Array`. If the accumulator is set to `acc = ""`,
* the result is usually a `String`.
*
* |===
*
* === Example
*
* This example returns the sum of the numeric values in the first input array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [2, 3] reduce ($ + $$)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* 5
* ----
*
* === Example
*
* This example adds the numbers in the `sum` example, concatenates the same
* numbers in `concat`, and shows that an empty array `[]` (defined in
* `myEmptyList`) returns `null` in `emptyList`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var myNums = [1,2,3,4]
* var myEmptyList = []
* output application/json
* ---
* {
*    "sum" : myNums reduce ($$ + $),
*    "concat" : myNums reduce ($$ ++ $),
*    "emptyList" : myEmptyList reduce ($$ ++ $)
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "sum": 10, "concat": "1234", "emptyList": null }
* ----
*
* === Example
*
* This example sets the first element from the first input array to `"z"`, and
* it adds `3` to the sum of the second input array. In `multiply`, it shows how
* to multiply each value in an array by the next
* (`[2,3,3] reduce ((item, acc) -> acc * item)`) to
* produce a final result of `18` (= `2 * 3 * 3`). The final example,
* `multiplyAcc`, sets the accumulator to `3` to multiply the result of
* `acc * item` (= `12`) by `3` (that is, `3 (2 * 2  * 3) = 36`), as shown in
* the output.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*    "concat" : ["a", "b", "c", "d"] reduce ((item, acc = "z") -> acc ++ item),
*    "sum": [0, 1, 2, 3, 4, 5] reduce ((item, acc = 3) -> acc + item),
*    "multiply" : [2,3,3] reduce ((item, acc) -> acc * item),
*    "multiplyAcc" : [2,2,3] reduce ((item, acc = 3) -> acc * item)
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "concat": "zabcd", "sum": 18, "multiply": 18, "multiplyAcc": 36 }
* ----
*
* === Example
*
* This example shows a variety of uses of `reduce`, including its application to
* arrays of boolean values and objects.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* var myVar =
* {
*   "a": [0, 1, 2, 3, 4, 5],
*   "b": ["a", "b", "c", "d", "e"],
*   "c": [{ "letter": "a" }, { "letter": "b" }, { "letter": "c" }],
*   "d": [true, false, false, true, true]
* }
* ---
* {
*   "a" : [0, 1, 2, 3, 4, 5] reduce $$,
*   "b": ["a", "b", "c", "d", "e"] reduce $$,
*   "c": [{ "letter": "a" }, { "letter": "b" }, { "letter": "c" }] reduce ((item, acc = "z") -> acc ++ item.letter),
*   "d": [{ letter: "a" }, { letter: "b" }, { letter: "c" }] reduce $$,
*   "e": [true, false, false, true, true] reduce ($$ and $),
*   "f": [true, false, false, true, true] reduce ((item, acc) -> acc and item),
*   "g": [true, false, false, true, true] reduce ((item, acc = false) -> acc and item),
*   "h": [true, false, false, true, true] reduce $$,
*   "i": myVar.a reduce ($$ + $),
*   "j": myVar.a reduce ((item, acc) -> acc + item),
*   "k": myVar.a reduce ((item, acc = 3) -> acc + item),
*   "l": myVar.a reduce $$,
*   "m": myVar.b reduce ($$ ++ $),
*   "n": myVar.b reduce ((item, acc) -> acc ++ item),
*   "o": myVar.b reduce ((item, acc = "z") -> acc ++ item),
*   "p": myVar.b reduce $$,
*   "q": myVar.c reduce ((item, acc = "z") -> acc ++ item.letter),
*   "r": myVar.c reduce $$,
*   "s": myVar.d reduce ($$ and $),
*   "t": myVar.d reduce ((item, acc) -> acc and item),
*   "u": myVar.d reduce ((item, acc = false) -> acc and item),
*   "v": myVar.d reduce $$,
*   "w": ([0, 1, 2, 3, 4] reduce ((item, acc = {}) -> acc ++ { a: item })) pluck $,
*   "x": [] reduce $$,
*   "y": [] reduce ((item,acc = 0) -> acc + item)
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* "a": 0,
* "b": "a",
* "c": "zabc",
* "d": { "letter": "a" },
* "e": false,
* "f": false,
* "g": false,
* "h": true,
* "i": 15,
* "j": 15,
* "k": 18,
* "l": 0,
* "m": "abcde",
* "n": "abcde",
* "o": "zabcde",
* "p": "a",
* "q": "zabc",
* "r": { "letter": "a" },
* "s": false,
* "t": false,
* "u": false,
* "v": true,
* "w": [ 0,1,2,3,4 ],
* "x": null,
* "y": 0
* }
* ----
**/
fun reduce <T>(@StreamCapable items: Array<T>, callback: (item: T, accumulator: T) -> T ): T | Null = native("system::ArrayReduceFunctionValue")
//Works like fold left
fun reduce <T,A>(@StreamCapable items: Array<T>, callback: (item: T, accumulator: A) -> A ): A = native("system::ArrayReduceFunctionValue")

/**
* Applies a reduction expression to the characters in a string.
*
*
* For each character of the input string, in order, `reduce` applies the reduction
* lambda expression (function), then replaces the accumulator with the new
* result. The lambda expression can use both the current character
* and the current accumulator value.
*
* Note that if the string is empty and no default value is set on the
* accumulator parameter, an empty string is returned.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The string to reduce.
* | callback | The function to apply.
* |===
*
* === Example
*
* This example shows how `reduce` can be used to reverse a string.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* "hello world" reduce (item, acc = "") -> item ++ acc
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* "dlrow olleh"
* ----
**/
fun reduce (@StreamCapable text: String, callback: (item: String, accumulator: String) -> String ): String = native("system::StringReduceFunctionValue")
fun reduce <A>(@StreamCapable text: String, callback: (item: String, accumulator: A) -> A ): A = native("system::StringReduceFunctionValue")

/**
* Helper function that enables `reduce` to work with a `null` value.
*/
fun reduce <T, A>(@StreamCapable items: Null, callback: (item: T, accumulator: A) -> A ): Null = null

/**
* Returns an object that groups items from an array based on specified
* criteria, such as an expression or matching selector.
*
*
* This version of `groupBy` groups the elements of an array using the
* `criteria` function. Other versions act on objects and handle null values.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | items | The array to group.
* | criteria | Expression providing the criteria by which to group the items in the array.
* |===
*
* === Example
*
* This example groups items from the input array `["a","b","c"]` by their
* indices. Notice that it returns the numeric indices as strings and that items
* (or values) of the array are returned as arrays, in this case, with a single
* item each. The items in the array are grouped based on an anonymous function
* `(item, index) -> index` that uses named parameters (`item` and `index`).
* Note that you can produce the same result using the anonymous parameter
* `&#36;&#36;` to identify the indices of the array like this:
* `["a","b","c"] groupBy &#36;&#36;`
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* ["a","b","c"] groupBy (item, index) -> index
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "2": [ "c" ], "1": [ "b" ], "0": [ "a" ] }
* ----
*
* === Example
*
* This example groups the elements of an array based on the language field.
* Notice that it uses the `item.language` selector to specify the grouping
* criteria. So the resulting object uses the "language" values (`"Scala"` and
* `"Java"`) from the input to group the output. Also notice that the output
* places the each input object in an array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var myArray = [
*    { "name": "Foo", "language": "Java" },
*    { "name": "Bar", "language": "Scala" },
*    { "name": "FooBar", "language": "Java" }
* ]
* output application/json
* ---
* myArray groupBy (item) -> item.language
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "Scala": [
*     { "name": "Bar", "language": "Scala" }
*   ],
*   "Java": [
*     { "name": "Foo", "language": "Java" },
*     { "name": "FooBar", "language": "Java" }
*   ]
* }
* ----
*
* === Example
*
* This example uses `groupBy "myLabels"`to return an object where `"mylabels"`
* is the key, and an array of selected values
* (`["Open New", "Zoom In", "Zoom Out", "Original View" ]`) is the value. It
* uses the selectors (`myVar.menu.items.*label`) to create that array. Notice
* that the selectors retain all values where `"label"` is the key but filter
* out values where `"id"` is the key.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var myVar = { menu: {
*     header: "Move Items",
*     items: [
*         {"id": "internal"},
*         {"id": "left", "label": "Move Left"},
*         {"id": "right", "label": "Move Right"},
*         {"id": "up", "label": "Move Up"},
*         {"id": "down", "label": "Move Down"}
*     ]
* }}
* output application/json
* ---
* (myVar.menu.items.*label groupBy "myLabels")
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "myLabels": [ "Move Left", "Move Right", "Move Up", "Move Down" ] }
* ----
*/
fun groupBy <T,R>(items: Array<T> , criteria: (item: T, index: Number) -> R): {|(R): Array<T>|} = native("system::ArrayGroupByFunctionValue")

/**
* Returns an object that groups characters from a string based on specified
* criteria, such as an expression or matching selector.
*
*
* This version of `groupBy` groups the elements of an array using the
* `criteria` function. Other versions act on objects and handle `null` values.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The string to group by.
* | criteria | The criteria to use.
* |===
*
* === Example
*
* This example shows howyou can use `groupBy` to split a string into
* vowels and not vowels.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* "hello world!" groupBy (not isEmpty($ find /[aeiou]/))
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "false": "hll wrld!",
*   "true": "eoo"
* }
* ----
**/
fun groupBy <R>(text: String , criteria: (character: String, index: Number) -> R): {(R): String} = do {
    fun toArray_priv(s: String): Array<String> = s splitBy "" filter not isEmpty($)
    fun fromArray_priv(s: Array<String>): String = s joinBy ""
    ---
    mapObject(
        groupBy(toArray_priv(text), criteria),
        (value, key, index) -> {(key): fromArray_priv(value)}
    )
}

/**
* Groups elements of an object based on criteria that the `groupBy`
* uses to iterate over elements in the input.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | object | The object containing objects to group.
* | criteria | The grouping criteria to apply to elements in the input object, such as a `key` and/or `value` of the object to use for grouping.
* |===
*
* === Example
*
* This example groups objects within an array of objects using the anonymous
* parameter `&#36;` for the value of each key in the input objects. It applies
* the DataWeave `upper` function to those values. In the output, these values
* become upper-case keys. Note that you can also write the same example using
* a named parameter for the within an anonymous function like this:
* `{ "a" : "b", "c" : "d"} groupBy (value) -> upper(value)`
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "a" : "b", "c" : "d"} groupBy upper($)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "D": { "c": "d" }, "B": { "a": "b" } }
* ----
*
* === Example
*
* This example uses `groupBy "costs"` to produce a JSON object from an XML object
* where `"costs"` is the key, and the selected values of the XML element `prices`
* becomes the JSON value (`{ "price": "9.99", "price": "10.99" }`).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var myRead =
* read("<prices><price>9.99</price><price>10.99</price></prices>","application/xml")
* output application/json
* ---
* myRead.prices groupBy "costs"
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "costs" : { "price": "9.99", "price": "10.99" } }
* ----
*/
fun groupBy <K,V,R>(object: {(K)?: V}, criteria: (value: V, key: K) -> R): {(R): {(K)?: V}} = native("system::ObjectGroupByFunctionValue")

/**
* Helper function that enables `groupBy` to work with a `null` value.
*/
fun groupBy(value: Null, criteria: (Nothing, Nothing) -> Any): Null = null

//REMOVE
/**
* Removes specified values from an input value.
*
*
* This version of `--` removes all instances of the specified items from an array. Other
* versions act on objects, strings, and the various date and time formats that
* are supported by DataWeave.
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | source | The array containing items to remove.
* | toRemove | Items to remove from the source array.
* |===
*
* === Example
*
* This example removes specified items from an array. Specifically, it removes
* all instances of the items listed in the array on the right side of `--` from
* the array on the left side of the function, leaving `[0]` as the result.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "a" : [0, 1, 1, 2] -- [1,2] }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a": [0] }
* ----
**/
@Labels(labels =["removeAll"])
fun -- <S>(source: Array<S> , toRemove: Array<Any>): Array<S> = native("system::ArrayRemoveFunctionValue")

/**
* Removes specified key-value pairs from an object.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | source | The source object (an `Object` type).
* | toRemove | Object that contains the key-value pairs to remove from the source object.
* |===
*
* === Example
*
* This example removes a key-value pair from the source object.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "hello" : "world", "name" : "DW" } -- { "hello" : "world"}
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "name": "DW" }
* ----
*/
@Labels(labels =["removeAll"])
fun -- <K,V>(source: {(K)?: V} , toRemove: Object): {(K)?:V} = native("system::ObjectRemoveFunctionValue")

/**
 * Removes all key-value pairs from the source object that match the specified search key.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name   | Description
 * | source | The source object (an `Object` type).
 * | toRemove | An array of keys to specify the key-value pairs to remove from the source object.
 * |===
 *
 * === Example
 *
 * This example removes two key-value pairs from the source object.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * output application/json
 * ---
 * { "yes" : "no", "good" : "bad", "old" : "new" } -- ["yes", "old"]
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * { "good": "bad" }
 * ----
 */
@Labels(labels =["removeAll"])
fun --(source: Object, keys: Array<String>) =
  keys reduce (key, obj = source) -> (obj - key)

/**
 * Removes specified key-value pairs from an object.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name   | Description
 * | source | The source object (an `Object` type).
 * | keys | A keys for the key-value pairs to remove from the source object.
 * |===
 *
 * === Example
 *
 * This example specifies the key-value pair to remove from the source object.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * output application/json
 * ---
 * { "hello" : "world", "name" : "DW" } -- ["hello" as Key]
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * { "name": "DW" }
 * ----
 */
@Labels(labels =["removeAll"])
fun --(source: Object, keys: Array<Key>) =
  keys reduce (key, obj = source) -> (obj - key)

/**
* Helper function that enables `--` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun --(source: Null, keys: Any) = null

/**
* Returns indices of an input that match a specified value.
*
*
* This version of the function returns indices of an array. Others return
* indices of a string.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | elements | An array with elements of any type.
* | elementToFind | Value to find in the input array.
* |===
*
* === Example
*
* This example finds the index of an element in a string array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* ["Bond", "James", "Bond"] find "Bond"
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [0,2]
* ----
*/
fun find <T>(@StreamCapable() elements: Array<T> , elementToFind: Any): Array<Number> = native("system::ArrayFindFunctionValue")

/**
* Returns the indices in the text that match the specified regular expression
* (regex), followed by the capture groups.
*
*
* The first element in each resulting sub-array is the index in the text that
* matches the regex, and the next ones are the capture groups in the regex
* (if present).
*
* Note: To retrieve parts of the text that match a regex. use the `scan` function.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | A string.
* | matcher | A Java regular expression for matching characters in the `text`.
* |===
*
* === Example
*
* This example finds the beginning and ending indices of words that contain `ea`
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* "I heart DataWeave" find /\w*ea\w*(\b)/
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ [2,7], [8,17] ]
* ----
*/
fun find(@StreamCapable() text: String , matcher: Regex): Array<Array<Number>> = native("system::StringFindRegexFunctionValue")

/**
* Lists indices where the specified characters of a string are present.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | A source string.
* | textToFind | The string to find in the source string.
* |===
*
* === Example
*
* This example lists the indices of "a" found in "aabccdbce".
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* "aabccdbce" find "a"
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [0,1]
* ----
*
**/
fun find(@StreamCapable() text: String, textToFind: String): Array<Number> = native("system::StringFindStringFunctionValue")

/**
* Helper function that enables `find` to work with a `null` value.
**/
fun find(@StreamCapable() text: Null, textToFind: Any): Array<Nothing> = []

/**
* Iterates over the input and returns the unique elements in it.
*
*
* DataWeave uses the result of the provided lambda as the
* uniqueness criteria.
*
* This version of `distinctBy` finds unique values in an array. Other versions
* act on an object and handle a `null` value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | items | The array to evaluate.
* | criteria | The criteria used to select an `item` and/or `index` from the array.
* |===
*
* === Example
*
* This example inputs an array that contains duplicate numbers and returns an
* array with unique numbers from that input. Note that you can write the same
* expression using an anonymous parameter for the values:
* `[0, 1, 2, 3, 3, 2, 1, 4] distinctBy &#36;`
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [0, 1, 2, 3, 3, 2, 1, 4] distinctBy (value) -> { "unique" : value }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ 0, 1, 2, 3, 4]
* ----
*
* === Example
*
* This example removes duplicates of `"Kurt Cagle"` from an array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* var record =  {
*   "title": "XQuery Kick Start",
*   "author": [
*     "James McGovern",
*     "Per Bothner",
*     "Kurt Cagle",
*     "James Linn",
*     "Kurt Cagle",
*     "Kurt Cagle",
*     "Kurt Cagle",
*     "Vaidyanathan Nagarajan"
*   ],
*   "year":"2000"
* }
* ---
* {
*     "book" : {
*       "title" : record.title,
*       "year" : record.year,
*       "authors" : record.author distinctBy $
*     }
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "book": {
*     "title": "XQuery Kick Start",
*     "year": "2000",
*     "authors": [
*       "James McGovern",
*       "Per Bothner",
*       "Kurt Cagle",
*       "James Linn",
*       "Vaidyanathan Nagarajan"
*     ]
*   }
* }
* ----
**/
fun distinctBy <T>(@StreamCapable items: Array<T>, criteria: (item: T, index: Number) -> Any): Array<T> = native("system::ArrayDistinctFunctionValue")

/**
* Removes duplicate key-value pairs from an object.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | object | The object from which to remove the key-value pairs.
* | criteria | The `key` and/or `value` used to identify the key-value pairs to remove.
* |===
*
* === Example
*
* This example inputs an object that contains duplicate key-value pairs and
* returns an object with key-value pairs from that input. Notice that the
* keys (`a` and `A`) are not treated with case sensitivity, but the values
* (`b` and `B`) are. Also note that you can write the same expression using
* an anonymous parameter for the values:
* `{a : "b", a : "b", A : "b", a : "B"} distinctBy &#36;`
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {a : "b", a : "b", A : "b", a : "B"} distinctBy (value) -> { "unique" : value }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a": "b", "a": "B" }
* ----
*
* === Example
*
* This example removes duplicates (`<author>James McGovern</author>`)
* from `<book/>`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/xml
* ---
* {
*    book : {
*      title : payload.book.title,
*      authors: payload.book.&author distinctBy $
*    }
* }
* ----
*
* ==== Input
*
* [source,XML,linenums]
* ----
* <book>
*   <title> "XQuery Kick Start"</title>
*   <author>James Linn</author>
*   <author>Per Bothner</author>
*   <author>James McGovern</author>
*   <author>James McGovern</author>
*   <author>James McGovern</author>
* </book>
* ----
*
* ==== Output
*
* [source,XML,linenums]
* ----
* <book>
*   <title> "XQuery Kick Start"</title>
*   <authors>
*       <author>James Linn</author>
*       <author>Per Bothner</author>
*       <author>James McGovern</author>
*   </authors>
* </book>
* ----
**/
fun distinctBy <K, V>(object: {(K)?: V}, criteria: (value: V, key: K) -> Any): Object = native("system::ObjectDistinctFunctionValue")

/**
* Helper function that enables `distinctBy` to work with a `null` value.
*/
fun distinctBy(@StreamCapable items: Null, criteria: (item: Nothing, index: Nothing) -> Any): Null = null

/**
* Returns a range with the specified boundaries.
*
*
* The upper boundary is inclusive.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | from | `Number` value that starts the range. The output includes the `from` value.
* | to | `Number` value that ends the range. The output includes the `from` value.
* |===
*
* === Example
*
* This example lists a range of numbers from 1 to 10.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "myRange": 1 to 10 }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "myRange": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10] }
* ----
*
* === Example
*
* DataWeave treats a string as an array of characters. This example applies `to` to a string.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var myVar = "Hello World!"
* output application/json
* ---
* {
*   indices2to6 : myVar[2 to 6],
*   indicesFromEnd : myVar[6 to -1],
*   reversal : myVar[11 to -0]
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "indices2to6": "llo W",
*   "indicesFromEnd": "World!",
*   "reversal": "!dlroW olleH"
* }
* ----
*/
fun to(from: Number , to: Number): Range = native("system::ToRangeFunctionValue")

//CONTAINS
/**
* Returns `true` if an input contains a given value, `false` if not.
*
*
* This version of `contains` accepts an array as input. Other versions
* accept a string and can use another string or regular expression to
* determine whether there is a match.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | items | The input array.
* | elements | Element to find in the array. Can be any supported data type.
* |===
*
* === Example
*
* This example finds that `2` is in the input array, so it returns `true`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ 1, 2, 3, 4 ] contains(2)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* true
* ----
*
* === Example
*
* This example indicates whether the input array contains '"3"'.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* ContainsRequestedItem: payload.root.*order.*items contains "3"
* ----
*
* ==== Input
*
* [source,XML,linenums]
* ----
* <?xml version="1.0" encoding="UTF-8"?>
* <root>
*     <order>
*       <items>155</items>
*     </order>
*     <order>
*       <items>30</items>
*     </order>
*     <order>
*       <items>15</items>
*     </order>
*     <order>
*       <items>5</items>
*     </order>
*     <order>
*       <items>4</items>
*       <items>7</items>
*     </order>
*     <order>
*       <items>1</items>
*       <items>3</items>
*     </order>
*     <order>
*         null
*     </order>
* </root>
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "ContainsRequestedItem": true }
* ----
**/
fun contains <T>(@StreamCapable items: Array<T> , element: Any): Boolean = native("system::ArrayContainsFunctionValue")

/**
* Indicates whether a string contains a given substring. Returns `true`
* or `false`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | An input string (a `String`).
* | toSearch | The substring (a `String`) to find in the input string.
* |===
*
* === Example
*
* This example finds "mule" in the input string "mulesoft", so it returns `true`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* "mulesoft" contains("mule")
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* true
* ----
*
* === Example
*
* This example finds that the substring `"me"` is in `"some string"`, so it
* returns `true`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { ContainsString : payload.root.mystring contains("me") }
* ----
*
* ==== Input
*
* [source,XML,linenums]
* ----
* <?xml version="1.0" encoding="UTF-8"?>
* <root><mystring>some string</mystring></root>
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "ContainsString": true }
* ----
**/
fun contains(text: String , toSearch: String): Boolean = native("system::StringStringContainsFunctionValue")

/**
* Returns `true` if a string contains a match to a regular expression, `false`
* if not.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | An input string.
* | matcher | A Java regular expression for matching characters in the input `text`.
* |===
*
* === Example
*
* This example checks for any of the letters `e` through `g` in the input
* `mulesoft`, so it returns `true`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* contains("mulesoft", /[e-g]/)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* true
* ----
*
* === Example
*
* This example finds a match to `/s[t|p]rin/` within `"A very long string"`,
* so it returns `true`. The `[t|p]` in the regex means `t` or `p`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* ContainsString: payload.root.mystring contains /s[t|p]rin/
* ----
*
* ==== Input
*
* [source,XML,linenums]
* ----
* <?xml version="1.0" encoding="UTF-8"?>
* <root><mystring>A very long string</mystring></root>
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "ContainsString": true }
* ----
**/
fun contains(text: String , matcher: Regex): Boolean = native("system::StringRegexContainsFunctionValue")

/**
* Helper function that enables `contains` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun contains(text: Null , matcher: Any): false = false

//ORDERBY
/**
* Reorders the elements of an input using criteria that acts on selected
* elements of that input.
*
*
* This version of `orderBy` takes an object as input. Other versions act on an
* input array or handle a `null` value.
*
* Note that you can reference the index with the anonymous parameter
* `&#36;&#36;` and the value with `&#36;`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | object | The object to reorder.
* | criteria | The result of the function is used as the criteria to reorder the object.
* |===
*
* === Example
*
* This example alphabetically orders the values of each object in the input
* array. Note that `orderBy($.letter)` produces the same result as
* `orderBy($[0])`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [{ letter: "e" }, { letter: "d" }] orderBy($.letter)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [
*   {
*     "letter": "d"
*   },
*   {
*     "letter": "e"
*   }
* ]
* ----
*
* === Example
*
* The `orderBy` function doesn't have an option to sort the result in a descending order. In these cases, invert the order of
* the resulting array using `[-1 to 0]`:
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* orderDescending: ([3,8,1] orderBy $)[-1 to 0]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "orderDescending": [8,3,1] }
* ----
**/
fun orderBy <K,V,R, O <: {(K)?: V}>(object: O, criteria: (value: V, key: K) -> R): O = native('system::ObjectOrderByFunctionValue')

/**
* Sorts an array using the specified criteria.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | array | The array to sort.
* | criteria | The result of the function serves as criteria for sorting the array. It should return a simple value (`String`, `Number`, and so on).
* |===
*
* === Example
*
* This example sorts an array of numbers based on the numeric values.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [3,2,3] orderBy $
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ 2, 3, 3 ]
* ----
*
* === Example
*
* This example sorts an array of people based on their age.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [{name: "Santiago", age: 42},{name: "Leandro", age: 29}, {name: "Mariano", age: 35}] orderBy (person) -> person.age
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [
*   {
*     name: "Leandro",
*     age: 29
*   },
*   {
*     name: "Mariano",
*     age: 35
*   },
*   {
*     name: "Santiago",
*     age: 42
*   }
* ]
* ----
*
* === Example
*
* This example sorts an array of DateTime in descending order.
* 
* ==== Source
* 
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [|2020-10-01T23:57:59.017Z|, |2022-12-22T12:12:12.011Z|, |2020-10-01T12:40:10.012Z|, |2020-10-01T23:57:59.021Z|]
*   orderBy -($ as Number {unit: "milliseconds"})
* ----
*
* ==== Output
* 
* [source,JSON,linenums]
* ----
* [
*   "2022-12-22T12:12:12.011Z",
*   "2020-10-01T23:57:59.021Z",
*   "2020-10-01T23:57:59.017Z",
*   "2020-10-01T12:40:10.012Z"
* ]
* ----
*
* === Example
*
* This example changes the order of the objects in a JSON array. The expression first orders them alphabetically by the value of the `Type` key, then reverses the order based on the `[-1 to 0]`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var myInput = [
*     {
*         "AccountNumber": "987999321",
*         "NameOnAccount": "QA",
*         "Type": "AAAA",
*         "CDetail": {
*             "Status": "Open"
*         }
*     },
*     {
*         "AccountNumber": "12399978",
*         "NameOnAccount": "QA",
*         "Type": "BBBB",
*         "CDetail": {}
*     },
*     {
*         "AccountNumber": "32199974",
*         "NameOnAccount": "QA",
*         "Type": "CCCC",
*         "CDetail": {}
*     }
* ]
* output application/json
* ---
* (myInput orderBy $.Type)[-1 to 0]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [
*   {
*     "AccountNumber": "32199974",
*     "NameOnAccount": "QA",
*     "Type": "CCCC",
*     "CDetail": {
*
*     }
*   },
*   {
*     "AccountNumber": "12399978",
*     "NameOnAccount": "QA",
*     "Type": "BBBB",
*     "CDetail": {
*
*     }
*   },
*   {
*     "AccountNumber": "987999321",
*     "NameOnAccount": "QA",
*     "Type": "AAAA",
*     "CDetail": {
*       "Status": "Open"
*     }
*   }
* ]
* ----
**/
fun orderBy <T,R>(array: Array<T> , criteria: (item: T, index: Number) -> R): Array<T> = native("system::ArrayOrderByFunctionValue")

/**
* Helper function that enables `orderBy` to work with a `null` value.
*/
fun orderBy(value: Null , criteria: (item: Nothing, index: Nothing) -> Null): Null = null

//UNARY OPERATORS
/**
* Returns the average of numbers listed in an array.
*
*
* An array that is empty or that contains a non-numeric value results
* in an error.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | values | The input array of numbers.
* |===
*
* === Example
*
* This example returns the average of multiple arrays.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { a: avg([1, 1000]), b: avg([1, 2, 3]) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a": 500.5, "b": 2 }
* ----
**/
fun avg(values: Array<Number>): Number = sum(values) / sizeOf(values)

/**
* Returns the highest `Comparable` value in an array.
*
*
* The items must be of the same type, or the function throws an error. The
* function returns `null` if the array is empty.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | values | The input array. The elements in the array can be any supported type.
* |===
*
* === Example
*
* This example returns the maximum value of each input array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { a: max([1, 1000]), b: max([1, 2, 3]), c: max([1.5, 2.5, 3.5]) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a": 1000, "b": 3, "c": 3.5 }
* ----
**/
fun max <T <: Comparable>(@StreamCapable values: Array<T>): T | Null = values maxBy $

/**
* Returns the lowest `Comparable` value in an array.
*
*
* The items must be of the same type or `min` throws an error. The function
* returns `null` if the array is empty.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | values | The input array. The elements in the array can be any supported type.
* |===
*
* === Example
*
* This example returns the lowest numeric value of each input array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { a: min([1, 1000]), b: min([1, 2, 3]), c: min([1.5, 2.5, 3.5]) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a": 1, "b": 1, "c": 1.5 }
* ----
**/
fun min <T <: Comparable>(@StreamCapable values: Array<T>): T | Null = values minBy $

/**
* Returns the sum of numeric values in an array.
*
*
* Returns `0` if the array is empty and produces an error when non-numeric
* values are in the array.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | values | The input array of numbers.
* |===
*
* === Example
*
* This example returns the sum of the values in the input array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* sum([1, 2, 3])
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* 6
* ----
*/
fun sum(@StreamCapable values: Array<Number>): Number = do {
  if (isDefaultOperatorDisabledExceptionHandling())
    values match {
      case [] -> 0
      else -> ($ reduce (value, acc) -> value + acc) as Number
    }
  else
    ((values reduce (value, acc) -> value + acc) default 0) as Number
}

//SIZEOF
/**
* Returns the number of elements in an array. It returns `0` if the array
* is empty.
*
*
* This version of `sizeOf` takes an array or an array of arrays as input.
* Other versions act on arrays of objects, strings, or binary values.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | array | The input array. The elements in the array can be any supported type.
* |===
*
* === Example
*
* This example counts the number of elements in the input array. It returns `3`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* sizeOf([ "a", "b", "c"])
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* 3
* ----
*
* === Example
*
* This example returns a count of elements in the input array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   "arraySizes": {
*      size3: sizeOf([1,2,3]),
*      size2: sizeOf([[1,2,3],[4]]),
*      size0: sizeOf([])
*    }
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*    "arraySizes": {
*      "size3": 3,
*      "size2": 2,
*      "size0": 0
*    }
* }
* ----
**/
fun sizeOf(array: Array<Any>): Number = native("system::ArraySizeOfFunctionValue")

/**
* Returns the number of key-value pairs in an object.
*
*
* This function accepts an array of objects. Returns `0` if the input object is
* empty.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | object | The input object that contains one or more key-value pairs.
* |===
*
* === Example
*
* This example counts the key-value pairs in the input object, so it returns `2`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* sizeOf({a: 1, b: 2})
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* 2
* ----
*
* === Example
*
* This example counts the key-value pairs in an object.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*    objectSizes : {
*      sizeIs2: sizeOf({a:1,b:2}),
*      sizeIs0: sizeOf({})
*    }
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "objectSize": {
*     "sizeIs2": 2,
*     "sizeIs0": 0
*   }
* }
* ----
**/
fun sizeOf(object: Object): Number = native("system::ObjectSizeOfFunctionValue")


/**
* Returns the number of elements in an array of binary values.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | binary | The input array of binary values.
* |===
*
* === Example
*
* This example returns the size of an array of binary values.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* sizeOf(["\u0000" as Binary, "\u0001" as Binary, "\u0002" as Binary])
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* 3
* ----
**/
fun sizeOf(binary: Binary): Number = native("system::BinarySizeOfFunctionValue")

/**
* Returns the number of characters (including white space) in an string.
*
*
* Returns `0` if the string is empty.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The input text.
* |===
*
* === Example
*
* This example returns the number of characters in the input string `"abc"`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* sizeOf("abc")
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* 3
* ----
*
* === Example
*
* This example returns the number of characters in the input strings. Notice it
* counts blank spaces in the string `"my string"`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   sizeOfSting2 : sizeOf("my string"),
*   sizeOfEmptyString: sizeOf("")
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "sizeOfSting2": 9,
*   "sizeOfEmptyString": 0
* }
* ----
**/
fun sizeOf(text: String): Number = native("system::StringSizeOfFunctionValue")

/**
* Returns the number of characters in a `Period` value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value  | The input `Period`.
* |===
*
* === Example
*
* This example returns the number of characters in the `Period` value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   a: sizeOf(|P3D|)
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a": 3 }
* ----
*
**/
@Since(version = "2.6.0")
fun sizeOf(value: Period): Number = do {
  sizeOf(value as String)
}

/**
* Returns the number of characters in a `DateTime` value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value  | The input `DateTime`.
* |===
*
* === Example
*
* This example returns the number of characters in the `DateTime` value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   a: sizeOf(|2025-07-13T18:06:59.314033Z|)
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a": 27 }
* ----
*
**/
@Since(version = "2.6.0")
fun sizeOf(value: DateTime): Number = do {
  sizeOf(value as String)
}

/**
* Returns the number of characters in a `LocalDateTime` value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value  | The input `LocalDateTime`.
* |===
*
* === Example
*
* This example returns the number of characters in the `LocalDateTime` value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   a: sizeOf(|2025-07-13T18:06:59.314033|)
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a": 26 }
* ----
*
**/
@Since(version = "2.6.0")
fun sizeOf(value: LocalDateTime): Number = do {
  sizeOf(value as String)
}

/**
* Returns the number of characters in a `Number` value.
*
* To keep backward compatibility with 2.4, returns `1` for any `Number` value when the flag `com.mulesoft.dw.legacySizeOfNumber` is enabled.
*
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | number | The input number.
* |===
*
* === Example
*
* This example returns the number of characters in the `Number` value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   a: sizeOf(123),
*   b: sizeOf(123.45)
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a": 3, "b": 6 }
* ----
*
* === Example
*
* This example shows how the `sizeOf` function works when the flag `com.mulesoft.dw.legacySizeOfNumber` is enabled.
* Notice it return 1 for any given input.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   a: sizeOf(123)
*   b: sizeOf(123.45)
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a": 1, "b": 1 }
* ----
**/
@Since(version = "2.6.0")
fun sizeOf(number: Number): Number = do {
  if (isLegacySizeOfNumber())
    1
  else
    sizeOf(number as String)
}

/**
* Helper function that enables `sizeOf` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun sizeOf(n: Null): Null = null

/**
* Turns a set of subarrays (such as `[ [1,2,3], [4,5,[6]], [], [null] ]`) into a single, flattened array (such as `[ 1, 2, 3, 4, 5, [6], null ]`).
*
*
* Note that it flattens only the first level of subarrays and omits empty subarrays.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | items | The input array of arrays made up of any supported types.
* |===
*
* === Example
*
* This example defines three arrays of numbers, creates another array containing those three arrays, and then uses the flatten function to convert the array of arrays into a single array with all values.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* var array1 = [1,2,3]
* var array2 = [4,5,6]
* var array3 = [7,8,9]
* var arrayOfArrays = [array1, array2, array3]
* ---
* flatten(arrayOfArrays)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ 1,2,3,4,5,6,7,8,9 ]
* ----
*
* === Example
*
* This example returns a single array from nested arrays of objects.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var myData =
* { user : [
*    {
*      group : "dev",
*      myarray : [
*        { name : "Shoki", id : 5678 },
*        { name : "Mariano", id : 9123 }
*      ]
*    },
*    {
*      group : "test",
*      myarray : [
*        { name : "Sai", id : 2001 },
*        { name : "Peter", id : 2002 }
*      ]
*    }
*  ]
* }
* output application/json
* ---
* flatten(myData.user.myarray)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [
*   {
*     "name": "Shoki",
*     "id": 5678
*   },
*   {
*     "name": "Mariano",
*     "id": 9123
*   },
*   {
*     "name": "Sai",
*     "id": 2001
*   },
*   {
*     "name": "Peter",
*     "id": 2002
*   }
* ]
* ----
*
* Note that
* if you use `myData.user.myarray` to select the array of objects in `myarray`,
* instead of using `flatten(myData.user.myarray)`, the output is a nested array of objects:
*
* [source,JSON,linenums]
* ----
* [
*   [
*     {
*       "name": "Shoki",
*       "id": 5678
*     },
*     {
*       "name": "Mariano",
*       "id": 9123
*     }
*   ]
* ]
* ----
**/
fun flatten <T, Q>(@StreamCapable items: Array<Array<T> | Q>): Array<T | Q> = native("system::ArrayFlattenFunctionValue")

/**
* Helper function that enables `flatten` to work with a `null` value.
*/
fun flatten (@StreamCapable value: Null): Null = null

/**
* Performs the opposite of `zip`. It takes an array of arrays as input.
*
*
* The function groups the values of the input sub-arrays by matching indices,
* and it outputs new sub-arrays with the values of those matching indices. No
* sub-arrays are produced for unmatching indices. For example, if one input
* sub-array contains four elements (indices 0-3) and another only contains
* three (indices 0-2), the function will not produce a sub-array for the
* value at index 3.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | items | The input array of arrays.
* |===
*
* === Example
*
* This example unzips an array of arrays. It outputs the first index of each
* sub-array into one array `[ 0, 1, 2, 3 ]`, and the second index of each into
* another `[ "a", "b", "c", "d" ]`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* unzip([ [0,"a"], [1,"b"], [2,"c"],[ 3,"d"] ])
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ [ 0, 1, 2, 3 ], [ "a", "b", "c", "d" ] ]
* ----
*
* === Example
*
* This example unzips an array of arrays. Notice that the number of elements in
* the input arrays is not all the same. The function creates only as many full
* sub-arrays as it can, in this case, just one.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* unzip([ [0,"a"], [1,"a","foo"], [2], [3,"a"] ])
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [0,1,2,3]
* ----
**/
fun unzip<T>(items: Array<Array<T>>):Array<Array<T>> = do {
    var minSize = min(items map sizeOf($)) default 0
    ---
    ((0 to minSize - 1) as Array<Number>) map ((i)-> items map (item) -> item[i])
}

/**
* Returns `true` if a string ends with a provided substring, `false` if not.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The input string (a `String`).
* | suffix | The suffix string to find at the end of the input string.
* |===
*
* === Example
*
* This example finds "no" (but not "to") at the end of "Mariano".
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ "Mariano" endsWith "no", "Mariano" endsWith "to" ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ true, false ]
* ----
**/
fun endsWith(text: String, suffix: String): Boolean = native("system::StringEndsWithFunctionValue")

/**
* Helper function that enables `endsWith` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun endsWith(text: Null, suffix: Any): false = false

/**
* Merges an array into a single string value and uses the provided string
* as a separator between each item in the list.
*
*
* Note that `joinBy` performs the opposite task of `splitBy`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | elements |  The input array.
* | separator | A `String` used to join elements in the list.
* |===
*
* === Example
*
* This example joins the elements with a hyphen (`-`).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "hyphenate" : ["a","b","c"] joinBy "-" }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "hyphenate": "a-b-c" }
* ----
**/
fun joinBy(@StreamCapable elements: Array<StringCoerceable>, separator: String): String = do {
  if (isDefaultOperatorDisabledExceptionHandling())
    elements  match {
      case [] -> ""
      case [head ~ tail] ->
        tail reduce ((item, accumulator = head as String) -> accumulator ++ separator ++ (item as String))
  } else
    elements
    map ((item, index) -> item as String)
    reduce ((item, accumulator) -> accumulator  ++ separator ++ item) default ""
}
/**
* Helper function that enables `joinBy` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun joinBy(n: Null, separator: Any): Null = null

/**
* Returns an array with all of the matches found in an input string.
*
*
* Each match is returned as an array that contains the complete match followed
* by any capture groups in your regular expression (if present).
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The input string to scan.
* | regex | A Java regular expression that describes the pattern match in
the `text`.
* |===
*
* === Example
*
* In this example, the `regex` describes a URL. It contains three capture
* groups within the parentheses, the characters before and after the period
* (`.`). It produces an array of matches to the input URL and the capture
* groups. It uses `flatten` to change the output from an array of arrays into
* a simple array. Note that a `regex` is specified within forward slashes (`//`).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* flatten("www.mulesoft.com" scan(/([w]*)\.([a-z]*)\.([a-z]*)/))
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ "www.mulesoft.com", "www", "mulesoft", "com" ]
* ----
*
* === Example
*
* In the example, the `regex` describes an email address. It contains two
* capture groups, the characters before and after the `@`. It produces an
* array matches to the email addresses and capture groups in the input string.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* "anypt@mulesoft.com,max@mulesoft.com" scan(/([a-z]*)@([a-z]*).com/)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [
*   [ "anypt@mulesoft.com", "anypt", "mulesoft" ],
*   [ "max@mulesoft.com", "max", "mulesoft" ]
* ]
* ----
**/
fun scan(text: String, matcher: Regex): Array<Array<String>> = native("system::StringScanFunctionValue")

/**
* Helper function that enables `scan` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun scan(text: Null, matcher: Any): Null = null

/**
* Splits a string into a string array based on a value that matches part of that
* string. It filters out the matching part from the returned array.
*
*
* This version of `splitBy` accepts a Java regular expression (regex) to
* match the input string. The regex can match any character in the input
* string. Note that `splitBy` performs the opposite operation of `joinBy`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The input string to split.
* | regex | A Java regular expression used to split the string. If it does not match some part of the string, the function will return the original, unsplit string in the array.
* |===
*
* === Example
*
* This example uses a Java regular expression to split an address block by the
* periods and forward slash in it. Notice that the regular expression goes
* between forward slashes.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* "192.88.99.0/24" splitBy(/[.\/]/)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* ["192", "88", "99", "0", "24"]
* ----
*
* === Example
*
* This example uses several regular expressions to split input strings. The
* first uses `/.b./` to split the string by `-b-`. The second uses `/\s/`
* to split by a space. The third example returns the original input string in
* an array (`[ "no match"]`) because the regex `/^s/` (for matching the first
* character if it is `s`) does not match the first character in the input
* string (`"no match"`). The fourth, which uses `/^n../`, matches the first
* characters in `"no match"`, so it returns `[ "", "match"]`. The last removes
* all numbers and capital letters from a string, leaving each of the lower case
* letters in the array. Notice that the separator is omitted from the output.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "splitters" : {
*    "split1" : "a-b-c" splitBy(/.b./),
*    "split2" : "hello world" splitBy(/\s/),
*    "split3" : "no match" splitBy(/^s/),
*    "split4" : "no match" splitBy(/^n../),
*    "split5" : "a1b2c3d4A1B2C3D" splitBy(/[0-9A-Z]/)
*   }
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   splitters: {
*     split1: [ "a", "c" ],
*     split2: [ "hello", "world" ],
*     split3: [ "no match" ],
*     split4: [ "", "match" ],
*     split5: [ "a", "b", "c", "d" ]
*   }
* }
* ----
*
* === Example
*
* This example splits the number by `.` and applies the index selector `[0]` to
* the result of the splitBy function. The splitBy returns `["192", "88", "99", "0"]`
* so the index * selector `[0]` just returns the first element in the array ("192").
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* ("192.88.99.0" splitBy("."))[0]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* "192"
* ----
*
* === Example
*
* This example uses a Java regular expression to split a string by `.` at every
* point the input string matches the regex. Note that the regular expression
* does not consider the periods between the backticks ```.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* 'root.sources.data.`test.branch.BranchSource`.source.traits' splitBy(/[.](?=(?:[^`]*`[^`]*`)*[^`]*$)/)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [
*  "root",
*  "sources",
*  "data",
*  "`test.branch.BranchSource`",
*  "source",
*  "traits"
]
* ----
**/
fun splitBy(text: String, regex: Regex): Array<String> = native("system::StringSplitStringFunctionValue")

/**
* Splits a string into a string array based on a separating string that matches
* part of the input string. It also filters out the matching string from the
* returned array.
*
*
* The separator can match any character in the input. Note that `splitBy` performs
* the opposite operation of `joinBy`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The string to split.
* | separator | A string used to separate the input string. If it does not match some part of the string, the function will return the original, unsplit string in the array.
* |===
*
* === Example
*
* This example splits a string containing an IP address by its periods.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* "192.88.99.0" splitBy(".")
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* ["192", "88", "99", "0"]
* ----
*
* === Example
*
* The first example (`splitter1`) uses a hyphen (`-`) in `"a-b-c"` to split the
* string. The second uses an empty string (`""`) to split each character
* (including the blank space) in the string. The third example splits based
* on a comma (`,`) in the input string. The last example does not split the
* input because the function is case sensitive, so the upper case `NO` does not
* match the lower case `no` in the input string.  Notice that the separator is
* omitted from the output.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "splitters" : {
*     "split1" : "a-b-c" splitBy("-"),
*     "split2" : "hello world" splitBy(""),
*     "split3" : "first,middle,last" splitBy(","),
*     "split4" : "no split" splitBy("NO")
*    }
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   splitters: {
*     split1: [ "a","b","c" ],
*     split2: [ "h","e","l","l","o","","w","o","r","l","d" ],
*     split3: [ "first","middle","last"],
*     split4: [ "no split"]
*   }
* }
* ----
**/
fun splitBy(text: String, separator: String): Array<String> = native("system::StringSplitRegexFunctionValue")

/**
* Helper function that enables `splitBy` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun splitBy(text: Null, separator: Any) = null

/**
* Returns `true` or `false` depending on whether the input string starts with a
* matching prefix.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The input string.
* | prefix | A string that identifies the prefix.
* |===
*
* === Example
*
* This example indicates whether the strings start with a given prefix.
* Note that you can use the `startsWith(text,prefix)` or
* `text startsWith(prefix)` notation (for example,
* `startsWith("Mari","Mar")` or `"Mari" startsWith("Mar")`).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ "Mari" startsWith("Mar"), "Mari" startsWith("Em") ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ true, false ]
* ----
**/
fun startsWith(text: String, prefix: String): Boolean = native("system::StringStartsWithFunctionValue")

/**
* Helper function that enables `startsWith` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun startsWith(text: Null, prefix: Any): false = false

/**
* Checks if an expression matches the entire input string.
*
*
* For use cases where you need to output or conditionally process the matched
* value, see
* https://docs.mulesoft.com/dataweave/latest/dataweave-pattern-matching[Pattern Matching in DataWeave].
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The input string.
* | matcher | A Java regular expression for matching characters in the string.
* |===
*
* === Example
*
* This example indicates whether the regular expression matches the input text.
* Note that you can also use the `matches(text,matcher)` notation (for example,
* `matches("admin123", /a.*\d+/)`).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ ("admin123" matches /a.*\d+/), ("admin123" matches /^b.+/) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ true, false ]
* ----
**/
fun matches(text: String, matcher: Regex): Boolean = native("system::StringMatchesFunctionValue")

/**
* Helper function that enables `matches` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun matches(text: Null, matcher: Any): false = false

/**
* Uses a Java regular expression (regex) to match a string and then separates it into
* capture groups. Returns the results in an array.
*
*
* Note that you can use `match` for pattern matching expressions that include
* https://docs.mulesoft.com/dataweave/latest/dataweave-pattern-matching[case
* statements].
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | A string.
* | matcher | A Java regex for matching characters in the `text`.
* |===
*
* === Example
*
* In this example, the regex matches the input email address and contains two
* capture groups within parentheses (located before and after the `@`). The
* result is an array of elements: The first matching the entire regex, the
* second matching the initial capture group (`[a-z]*`) in the the regex, the
* third matching the last capture group (`[a-z]*`).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* "me@mulesoft.com" match(/([a-z]*)@([a-z]*)\.com/)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [
*   "me@mulesoft.com",
*   "me",
*   "mulesoft"
* ]
* ----
*
* === Example
*
* This example outputs matches to values in an array that end in `4`. It uses
* `flatMap` to iterate over and flatten the list.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var a = '192.88.99.0/24'
* var b = '192.168.0.0/16'
* var c = '192.175.48.0/24'
* output application/json
* ---
* [ a, b, c ] flatMap ( $ match(/.*[$4]/) )
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [  "192.88.99.0/24", "192.175.48.0/24" ]
* ----
*
**/
fun match(text: String, matcher: Regex): Array<String> = native("system::StringRegexMatchFunctionValue")

/**
* Helper function that enables `match` to work with a `null` value.
*/
@Since(version = "2.4.0")
fun match(text: Null, matcher: Any) : Null = null

/**
* Returns the provided string in lowercase characters.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The input string.
* |===
*
* === Example
*
* This example converts uppercase characters to lower-case.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "name" : lower("MULESOFT") }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "name": "mulesoft" }
* ----
**/
fun lower(text: String): String = native("system::StringLowerFunctionValue")

/**
* Helper function that enables `lower` to work with a `null` value.
*/
fun lower(value:Null) : Null = null

/**
* Removes any blank spaces from the beginning and end of a string.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The string from which to remove any blank spaces.
* |===
*
* === Example
*
* This example trims a string. Notice that it does not remove any spaces from
* the middle of the string, only the beginning and end.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "trim": trim("   my really long  text     ") }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "trim": "my really long  text" }
* ----
*
* === Example
*
* This example shows how `trim` handles a variety strings and how it
* handles a null value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   "null": trim(null),
*   "empty": trim(""),
*   "blank": trim("     "),
*   "noBlankSpaces": trim("abc"),
*   "withSpaces": trim("    abc    ")
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "null": null,
*   "empty": "",
*   "blank": "",
*   "noBlankSpaces": "abc",
*   "withSpaces": "abc"
* }
* ----
**/
fun trim(text: String): String = native("system::StringTrimFunctionValue")

/**
* Helper function that enables `trim` to work with a `null` value.
*/
fun trim(value: Null): Null = null

/**
* Returns the provided string in uppercase characters.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The string to convert to uppercase.
* |===
*
* === Example
*
* This example converts lowercase characters to uppercase.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "name" : upper("mulesoft") }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "name": "MULESOFT" }
* ----
**/
fun upper(text: String): String = native("system::StringUpperFunctionValue")

/**
* Helper function that enables `upper` to work with a `null` value.
*/
fun upper(value: Null): Null = null

/**
* Raises the value of a `base` number to the specified `power`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | base |  A number (`Number` type) that serves as the base.
* | power |  A number (`Number` type) that serves as the power.
* |===
*
* === Example
*
* This example raises the value a `base` number to the specified `power`.
* Note that you can also use the `pow(base,power)` notation (for example,
* `pow(2,3)` to return `8`).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ (2 pow 3), (3 pow 2), (7 pow 3) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ 8, 9, 343 ]
* ----
**/
fun pow(base: Number, power: Number): Number = native("system::PowNumberFunctionValue")

/**
* Returns the modulo (the remainder after dividing the `dividend`
* by the `divisor`).
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | dividend |  The number that serves as the dividend for the operation.
* | divisor |  The number that serves as the divisor for the operation.
* |===
*
* === Example
*
* This example returns the modulo of the input values. Note that you can also
* use the `mod(dividend, divisor)` notation (for example, `mod(3, 2)` to return
* `1`).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ (3 mod 2), (4 mod 2), (2.2 mod 2) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ 1, 0, 0.2]
* ----
**/
fun mod(dividend: Number, divisor: Number): Number = native("system::ModuleNumberFunctionValue")

/**
* Returns the square root of a number.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | number | The number to evaluate.
* |===
*
* === Example
*
* This example returns the square root of a number.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ sqrt(4), sqrt(25), sqrt(100) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ 2, 5, 10 ]
* ----
**/
fun sqrt(number: Number): Number = native("system::SqrtNumberFunctionValue")

/**
* Returns the absolute value of a number.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | number | The number to evaluate.
* |===
*
* === Example
*
* This example returns the absolute value of the specified numbers.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ abs(-2), abs(2.5), abs(-3.4), abs(3) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ 2, 2.5, 3.4, 3 ]
* ----
**/
fun abs(number: Number): Number = native("system::AbsNumberFunctionValue")

/**
* Rounds a number up to the nearest whole number.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | number | The number to round.
* |===
*
* === Example
*
* This example rounds numbers up to the nearest whole numbers. Notice that `2.1`
* rounds up to `3`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
*
* [ ceil(1.5), ceil(2.1), ceil(3) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ 2, 3, 3 ]
* ----
**/
fun ceil(number: Number): Number = native("system::CeilNumberFunctionValue")

/**
* Rounds a number down to the nearest whole number.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | number | The number to evaluate.
* |===
*
* === Example
*
* This example rounds numbers down to the nearest whole numbers. Notice that
* `1.5` rounds down to `1`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ floor(1.5), floor(2.2), floor(3) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ 1, 2, 3]
* ----
**/
fun floor(number: Number): Number = native("system::FloorNumberFunctionValue")

/**
* Returns the primitive data type of a value, such as `String`.
*
*
* A value's type is taken from its runtime representation and is never one of
* the arithmetic types (intersection, union, `Any`, or `Nothing`) nor a type
* alias. If present, metadata of a value is included in the result of
* `typeOf` (see https://docs.mulesoft.com/dataweave/latest/dw-types-functions-metadataof[metadataOf]).
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | Input value to evaluate.
* |===
*
* === Example
*
* This example identifies the type of several input values.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ typeOf("A b"), typeOf([1,2]), typeOf(34), typeOf(true), typeOf({ a : 5 }) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ "String", "Array", "Number", "Boolean", "Object" ]
* ----
*
* === Example
*
* This example shows that the type of a value is independent of the type with
* which it is declared.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
*
* var x: String | Number = "clearly a string"
* var y: "because" = "because"
* ---
* [typeOf(x), typeOf(y)]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* ["String", "String"]
* ----
*/
fun typeOf <T>(value: T): Type<T> = native("system::TypeOfAnyFunctionValue")

/**
* Rounds a number up or down to the nearest whole number.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | number | The number to evaluate.
* |===
*
* === Example
*
* This example rounds decimal numbers to the nearest whole numbers.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ round(1.2), round(4.6), round(3.5) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ 1, 5, 4 ]
* ----
**/
fun round(number: Number): Number = native("system::RoundNumberFunctionValue")

/**
* Returns `true` if the given input value is empty, `false` if not.
*
*
* This version of `isEmpty` acts on an array. Other versions
* act on a string or object, and handle null values.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | elements | The input array to evaluate.
* |===
*
* === Example
*
* This example indicates whether the input array is empty.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ isEmpty([]), isEmpty([1]) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ true, false ]
* ----
*/
fun isEmpty(elements: Array<Any>): Boolean = native("system::EmptyArrayFunctionValue")

/**
* Returns `true` if the input string is empty, `false` if not.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | A string to evaluate.
* |===
*
* === Example
*
* This example indicates whether the input strings are empty.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ isEmpty(""), isEmpty("DataWeave") ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ true, false ]
* ----
*/
fun isEmpty(value: String): Boolean = native("system::EmptyStringFunctionValue")

/**
* Returns `true` if the given object is empty, `false` if not.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | The object to evaluate.
* |===
*
* === Example
*
* This example indicates whether the input objects are empty.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ isEmpty({}), isEmpty({name: "DataWeave"}) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ true, false ]
* ----
*/
fun isEmpty(value: Object): Boolean = native("system::EmptyObjectFunctionValue")

/**
* Returns `true` if the input is `null`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | `null` is the value in this case.
* |===
*
* === Example
*
* This example indicates whether the input is `null`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { "nullValue" : isEmpty(null) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "nullValue": true }
* ----
*/
fun isEmpty(value: Null): true = true

/**
* Returns `true` if it receives a date for a leap year, `false` if not.
*
*
* This version of `leapYear` acts on a `DateTime` type. Other versions act on
* the other date and time formats that DataWeave supports.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | dateTime | The `DateTime` value to evaluate.
* |===
*
* === Example
*
* This example indicates whether the input is a leap year.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ isLeapYear(|2016-10-01T23:57:59|), isLeapYear(|2017-10-01T23:57:59|) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ true, false ]
* ----
*/
fun isLeapYear(dateTime: DateTime): Boolean = native("system::LeapDateTimeFunctionValue")

/**
* Returns `true` if the input `Date` is a leap year, 'false' if not.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | date | The `Date` value to evaluate.
* |===
*
* === Example
*
* This example indicates whether the input is a leap year.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ isLeapYear(|2016-10-01|), isLeapYear(|2017-10-01|) ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ true, false ]
* ----
*/
fun isLeapYear(date: Date): Boolean = native("system::LeapLocalDateFunctionValue")

/**
* Returns `true` if the input local date-time is a leap year, 'false' if not.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | datetime | A `LocalDateTime` value to evaluate.
* |===
*
* === Example
*
* This example indicates whether the input is a leap year. It uses a `map`
* function to iterate through the array of its `LocalDateTime` values,
* applies the `isLeapYear`  to those values, returning the results in an array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ |2016-10-01T23:57:59-03:00|, |2016-10-01T23:57:59Z| ] map isLeapYear($)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ true, true ]
* ----
*/
fun isLeapYear(datetime: LocalDateTime): Boolean = native("system::LeapLocalDateTimeFunctionValue")

/**
* Returns `true` if the given number contains a decimal, `false` if not.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | number | The number to evaluate.
* |===
*
* === Example
*
* This example indicates whether a number has a decimal. Note that
* numbers within strings get coerced to numbers.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [ isDecimal(1.1), isDecimal(1), isDecimal("1.1") ]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ true, false, true ]
* ----
*/
fun isDecimal(number: Number): Boolean = native("system::DecimalNumberFunctionValue")

/**
* Returns `true` if the given number is an integer (which lacks decimals),
* `false` if not.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | number | The number to evaluate.
* |===
*
* === Example
*
* This example indicates whether the input is an integer for different values. Note numbers within
* strings get coerced to numbers.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* [isInteger(1), isInteger(2.0), isInteger(2.2), isInteger("1")]
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* [ true, true, false, true ]
* ----
*/
fun isInteger(number: Number): Boolean = native("system::IntegerNumberFunctionValue")

/**
* Returns `true` if the given string is empty (`""`), completely composed of whitespaces, or `null`. Otherwise, the function returns `false`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | An input string to evaluate.
* |===
*
* === Example
*
* This example indicates whether the given values are blank. It also uses the `not` and `!` operators to check that a value is not blank.
* The `!` operator is supported starting in Dataweave 2.2.0. Use `!` only in Mule 4.2 and later versions.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output  application/json
* var someString = "something"
* var nullString = null
* ---
* {
*   // checking if the string is blank
*   "emptyString" : isBlank(""),
*   "stringWithSpaces" : isBlank("      "),
*   "textString" : isBlank(someString),
*   "somePayloadValue" : isBlank(payload.nonExistingValue),
*   "nullString" : isBlank(nullString),
*
*   // checking if the string is not blank
*   "notEmptyTextString" : not isBlank(" 1234"),
*   "notEmptyTextStringTwo" : ! isBlank("")
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "emptyString": true,
*   "stringWithSpaces": true,
*   "textString": false,
*   "somePayloadValue": true,
*   "nullString": true,
*   "notEmptyTextString": true,
*   "notEmptyTextStringTwo": false
* }
* ----
*/
fun isBlank(text: String | Null): Boolean = isEmpty(trim(text))

/**
* Returns `true` if the number or numeric result of a mathematical operation is
* odd, `false` if not.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | number | A number to evaluate.
* |===
*
* === Example
*
* This example indicates whether the numbers are odd.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output  application/json
* ---
* { "isOdd" : [ isOdd(0), isOdd(1), isOdd(2+2) ] }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "isOdd": [ false, true, false ] }
* ----
*/
fun isOdd(number: Number): Boolean =  mod(number, 2) != 0

/**
* Returns `true` if the number or numeric result of a mathematical operation is
* even, `false` if not.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | number | The number to evaluate.
* |===
*
* === Example
*
* This example indicates whether the numbers and result of an operation
* are even.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output  application/json
* ---
* { "isEven" : [ isEven(0), isEven(1), isEven(1+1) ] }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "isEven" : [ true, false, true ] }
* ----
*/
fun isEven(number: Number): Boolean =
  mod(number, 2) == 0

/**
* Iterates over an array to return the lowest value of
* comparable elements from it.
*
*
* The items need to be of the same type. `minBy` returns an error if they are
* not, and it returns null when the array is empty.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | item | Element in the input array (of type `Number`, `Boolean`, `DateTime`, `LocalDateTime`, `Date`, `LocalTime`, `Time`, or `TimeZone`). Can be referenced with `&#36;`.
* |===
*
* === Example
*
* This example returns the lowest numeric value within objects
* (key-value pairs) in an array. Notice that it uses `item.a` to select the
* value of the object. You can also write the same expression like this, using
* an anonymous parameter:
* `[ { "a" : 1 }, { "a" : 3 }, { "a" : 2 } ] minBy &#36;.a`
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output  application/json
* ---
* [ { "a" : 1 }, { "a" : 2 }, { "a" : 3 } ] minBy (item) -> item.a
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a" : 1 }
* ----
*
* === Example
*
* This example gets the latest `DateTime`, `Date`, and `Time` from inputs
* defined in the variables `myDateTime1` and `myDateTime2`. It also shows that
* the function returns null on an empty array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var myDateTime1 = "2017-10-01T22:57:59-03:00"
* var myDateTime2 = "2018-10-01T23:57:59-03:00"
* output application/json
* ---
* {
*   myMinBy: {
*     byDateTime: [ myDateTime1, myDateTime2 ] minBy ((item) -> item),
*     byDate: [ myDateTime1 as Date, myDateTime2 as Date ] minBy ((item) -> item),
*     byTime: [ myDateTime1 as Time, myDateTime2 as Time ] minBy ((item) -> item),
*     aBoolean: [ true, false, (0 > 1), (1 > 0) ] minBy $,
*     emptyArray: [] minBy ((item) -> item)
*   }
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "myMinBy": {
*     "byDateTime": "2017-10-01T22:57:59-03:00",
*     "byDate": "2017-10-01",
*     "byTime": "22:57:59-03:00",
*     "aBoolean": false,
*     "emptyArray": null
*   }
* }
* ----
*
*/
fun minBy<T>(@StreamCapable array: Array<T>, criteria: (item: T) -> Comparable): T | Null =
  reduce(array, (val, prev) ->
    if(criteria(val) < criteria(prev))
      val
    else
      prev
  )

/**
* Iterates over an array and returns the highest value of
* `Comparable` elements from it.
*
*
* The items must be of the same type. `maxBy` throws an error if they are not,
* and the function returns `null` if the array is empty.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | array | The input array.
* | criteria | Expression for selecting an item from the array, where the item is a `Number`, `Boolean`, `DateTime`, `LocalDateTime`, `Date`, `LocalTime`, `Time`, or `TimeZone` data type. Can be referenced with `&#36;`.
* |===
*
* === Example
*
* This example returns the greatest numeric value within objects
* (key-value pairs) in an array. Notice that it uses `item.a` to select the
* value of the object. You can also write the same expression like this, using
* an anonymous parameter:
* `[ { "a" : 1 }, { "a" : 3 }, { "a" : 2 } ] maxBy &#36;.a`
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output  application/json
* ---
* [ { "a" : 1 }, { "a" : 3 }, { "a" : 2 } ] maxBy ((item) -> item.a)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "a" : 3 }
* ----
*
* === Example
*
* This example gets the latest `DateTime`, `Date`, and `Time` from inputs
* defined in the variables `myDateTime1` and `myDateTime2`. It also shows that
* the function returns null on an empty array.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* var myDateTime1 = "2017-10-01T22:57:59-03:00"
* var myDateTime2 = "2018-10-01T23:57:59-03:00"
* output application/json
* ---
* {
*   myMaxBy: {
*     byDateTime: [ myDateTime1, myDateTime2 ] maxBy ((item) -> item),
*     byDate: [ myDateTime1 as Date, myDateTime2 as Date ] maxBy ((item) -> item),
*     byTime: [ myDateTime1 as Time, myDateTime2 as Time ] maxBy ((item) -> item),
*     emptyArray: [] maxBy ((item) -> item)
*   }
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "myMaxBy": {
*     "byDateTime": "2018-10-01T23:57:59-03:00",
*     "byDate": "2018-10-01",
*     "byTime": "23:57:59-03:00",
*     "emptyArray": null
*   }
* }
* ----
*
*/
fun maxBy<T>(@StreamCapable array: Array<T>, criteria: (item: T) -> Comparable): T | Null =
  reduce(array, (val, prev) ->
    if (criteria(val) > criteria(prev))
      val
    else
      prev
  )

/**
* Returns the number of days between two dates.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | from | From date (a `Date` type).
* | to | To date (a `Date` type). Note that if the `to` date is _earlier_ than the `from` date, the function returns a negative number equal to the number of days between the two dates.
* |===
*
* === Example
*
* This example returns the number of days between the specified dates.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* { days : daysBetween('2016-10-01T23:57:59-03:00', '2017-10-01T23:57:59-03:00') }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "days" : 365 }
* ----
*/
fun daysBetween(from: Date, to: Date): Number = native("system::daysBetween")

/**
* Returns the index of the _first_ occurrence of the specified element in this array, or `-1` if this list does not contain the element.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | array | The array of elements to search.
* | value | The value to search.
* |===
*
* === Example
*
* This example shows how `indexOf` behaves given different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   present: ["a","b","c","d"] indexOf "c",
*   notPresent: ["x","w","x"] indexOf "c",
*   presentMoreThanOnce: ["a","b","c","c"] indexOf "c",
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "present": 2,
*    "notPresent": -1,
*    "presentMoreThanOnce": 2
*  }
* ----
**/
@Since(version = "2.4.0")
fun indexOf(array: Array, value: Any): Number =
     find(array, value)[0] default -1

/**
* Returns the index of the *first* occurrence of the specified String in this String.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | theString | The string to search.
* | search | The string to find within `theString`.
* |===
*
* === Example
*
* This example shows how the `indexOf` behaves under different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   present: "abcd" indexOf "c",
*   notPresent: "xyz" indexOf "c",
*   presentMoreThanOnce: "abcdc" indexOf "c",
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "present": 2,
*    "notPresent": -1,
*    "presentMoreThanOnce": 2
*  }
* ----
**/
@Since(version = "2.4.0")
fun indexOf(theString: String, search: String): Number =
     find(theString, search)[0] default -1

/**
* Helper method to make indexOf null friendly
**/
@Since(version = "2.4.0")
fun indexOf(array: Null, value: Any): Number =
      -1


/**
* Returns the index of the _last_ occurrence of the specified element in a given
* array or `-1` if the array does not contain the element.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | array | The array of elements to search.
* | value | The value to search.
* |===
*
* === Example
*
* This example shows how `indexOf` behaves given different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   present: ["a","b","c","d"] lastIndexOf "c",
*   notPresent: ["x","w","x"] lastIndexOf "c",
*   presentMoreThanOnce: ["a","b","c","c"] lastIndexOf "c",
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "present": 2,
*   "notPresent": -1,
*   "presentMoreThanOnce": 3
* }
* ----
**/
@Since(version = "2.4.0")
fun lastIndexOf(array: Array, value: Any): Number =
     find(array, value)[-1] default -1


/**
* Takes a string as input and returns the index of the _last_ occurrence of
* a given search string within the input. The function returns `-1` if the
* search string is not present in the input.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | string | The string to search.
* | value | A string value to search for within the input string.
* |===
*
* === Example
*
* This example shows how the `indexOf` behaves given different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*   present: "abcd" lastIndexOf "c",
*   notPresent: "xyz" lastIndexOf "c",
*   presentMoreThanOnce: "abcdc" lastIndexOf "c",
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "present": 2,
*   "notPresent": -1,
*   "presentMoreThanOnce": 4
* }
* ----
**/
@Since(version = "2.4.0")
fun lastIndexOf(array: String, value: String): Number =
     find(array, value)[-1] default -1

/**
* Helper function that enables `lastIndexOf` to work with a `null` value.
**/
@Since(version = "2.4.0")
fun lastIndexOf(array: Null, value: Any): Number =
      -1

/**
* Helper function that enables `then` to work with a `null` value.
**/
@Since(version = "2.4.0")
fun then(value: Null, callback: (previousResult: Nothing) -> Any): Null =
    value

/**
* This function works as a pipe that passes the value returned from the
* preceding expression to the next (a callback) only if the value returned
* by the preceding expression is not `null`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | previous | The value of the preceding expression.
* | callback | Callback that processes the result of `previous` if the result is not `null`.
* |===
*
* === Example
*
* This example shows how to use `then` to chain and continue processing
* the result of the previous expression.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*     "chainResult": ["mariano", "de Achaval"]
*             reduce ((item, accumulator) -> item ++ accumulator)
*             then ((result) -> sizeOf(result)),
*     "referenceResult" : ["mariano", "de Achaval"]
*                          map ((item, index) -> upper(item))
*                          then {
*                             name: $[0],
*                             lastName: $[1],
*                             length: sizeOf($)
*                         },
*     "onNullReturnNull": []
*                 reduce ((item, accumulator) -> item ++ accumulator)
*                 then ((result) -> sizeOf(result))
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "chainResult": 17,
*    "referenceResult": {
*      "name": "MARIANO",
*      "lastName": "DE ACHAVAL",
*      "length": 2
*    },
*    "onNullReturnNull": null
*  }
* ----
**/
@Since(version = "2.4.0")
@GlobalDescription
fun then<T, R>(previous: T, callback: (result: T) -> R): R =
    callback(previous)


/**
* Executes a callback function if the preceding expression returns a `null`
* value and then replaces the `null` value with the result of the callback.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | previous | The value of the preceding expression.
* | callback | Callback that generates a new value if `previous` returns `null`.
* |===
*
* === Example
*
* This example shows how `onNull` behaves when it receives a `null` value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* output application/json
* ---
* {
*      "onNull": []
*              reduce ((item, accumulator) -> item ++ accumulator)
*              then ((result) -> sizeOf(result))
*              onNull "Empty Text"
*  }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "onNull": "Empty Text"
* }
* ----
**/
@Since(version = "2.4.0")
fun onNull<R>(previous: Null, callback:() -> R): R =
        callback()

/**
* Helper function that enables `onNull` to work with a _non-null_ value.
**/
@Since(version = "2.4.0")
fun onNull<T>(previous: T, callback:() -> Any): T =
        previous
