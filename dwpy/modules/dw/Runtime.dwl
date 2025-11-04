/**
* This module contains functions for interacting with the DataWeave runtime, which executes the language.
*
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::Runtime` to the header of your
* DataWeave script.
*/
%dw 2.0

/**
 * Throws an exception with the specified message.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | message | An error message (`String`).
 * |===
 *
 * === Example
 *
 * This example returns a failure message `Data was empty` because the expression
 * `(sizeOf(myVar) &lt;= 0)` is `true`. A shortened version of the error message
 * is shown in the output.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::Runtime
 * var result = []
 * output application/json
 * ---
 * if(sizeOf(result) <= 0) fail('Data was empty') else result
 * ----
 *
 * ==== Output
 *
 * [source,TXT,linenums]
 * ----
 * ERROR 2018-07-29 11:47:44,983 ...
 * *********************************
 * Message               : "Data was empty
 * ...
 * ----
 */
fun fail (message: String = 'Error'): Nothing = native("system::fail")

/**
 * Produces an error with the specified message if the expression in
 * the evaluator returns `true`. Otherwise, the function returns the value.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | value | The value to return only if the `evaluator` expression is `false`.
 * | evaluator | Expression that returns `true` or `false`.
 * |===
 *
 * === Example
 *
 * This example produces a runtime error (instead of a SUCCESS message) because
 * the expression `isEmpty(result)` is `true`. It is `true` because an empty
 * object is passed through variable `result`.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import failIf from dw::Runtime
 * var result = {}
 * output application/json
 * ---
 * { "result" : "SUCCESS" failIf (isEmpty(result)) }
 * ----
 *
 * ==== Output
 *
 * [source,TXT,linenums]
 * ----
 * ERROR 2018-07-29 11:56:39,988 ...
 * **********************************
 * Message               : "Failed
 * ----
 */
fun failIf <T>(value: T, evaluator: (value: T) -> Boolean, message: String = 'Failed'): T =
    if(evaluator(value)) fail(message) else value

/**
* A String representation of a MIME type.
*/
@Experimental()
type MimeType = String

/**
* Input to the DataWeave reader created for the specified MIME type, which includes
* the Binary input and MIME type, as well as optional encoding and properties values.
*
* * `value`: The input, in Binary format.
* * `encoding`: The encoding for the reader to use.
* * `properties`: The reader properties used to parse the input.
* * `mimeType`: The MIME type of the input.
*/
@Experimental()
type ReaderInput = {
    value: Binary,
    encoding?: String,
    properties?: Dictionary<SimpleType>,
    mimeType: MimeType
}

/**
* Data type of the data that returns when a `run` function executes successfully.
*/
@Experimental()
type RunSuccess = {
   value: Binary,
   mimeType: MimeType,
   encoding?: String,
   logs : Array<LogEntry>
}

/**
* Data type of the data that returns when a `run` function executes successfully.
*/
@Since(version = "2.7.0")
@Experimental()
type RunResult = Result<RunSuccess, ExecutionFailure>

/**
* Data type of the data that returns when an `eval` function executes successfully.
*/
@Experimental()
type EvalSuccess = {
   value: Any,
   logs : Array<LogEntry>
}

/**
* Data type of the data that returns when an `eval` function executes successfully.
*/
@Since(version = "2.7.0")
@Experimental()
type EvalResult = Result<EvalSuccess, ExecutionFailure>

/**
* Data type of the data that returns when a `run` or `eval` function fails.
*/
@Experimental()
type ExecutionFailure = {
   message: String,
   kind: String,
   stack?: Array<String>,
   location: Location,
   logs : Array<LogEntry>
}

/**
* Type that represents the location of an expression in a DataWeave file.
*/
@Experimental()
type Location =  {
      start?: Position,
      end?: Position,
      locationString: String,
      text?: String,
      sourceIdentifier?: String,
}

/**
* Type that represents a position in a file by its index and its line and column.
*/
@Experimental()
type Position = {
    index: Number,
    line: Number,
    column: Number
}

/**
* Identifies the different kinds of log levels (`INFO`, `ERROR`, or `WARN`).
*/
@Experimental()
type LogLevel = "INFO" | "ERROR" | "WARN"

/**
* Type for a log entry, which consists of a `level` for a `LogLevel` value,
a `timestamp`, and `message`.
*/
@Experimental()
type LogEntry = {
    level: LogLevel,
    timestamp: String,
    message: String
}

/**
* Type that describes a data format property. The fields include a `name`,
* `description`, array of possible values (`possibleValues`), an optional default
* value (`defaultValue`), and an `optional` flag that indicates whether the property
* is required or not.
*/
@Experimental()
type DataFormatProperty = {
    name: String,
    optional: Boolean,
    defaultValue?: Any,
    description: String,
    possibleValues: Array<Any>
}

/**
* Description of a `DataFormat` that provides all metadata information.
*/
@Experimental()
type DataFormatDescriptor = {
    name: String,
    binary: Boolean,
    defaultEncoding?: String,
    extensions: Array<String>,
    defaultMimeType: String,
    acceptedMimeTypes: Array<String>,
    readerProperties: Array<DataFormatProperty>,
    writerProperties: Array<DataFormatProperty>
}

/**
* Function that is called when a privilege must be granted to the current execution.
*
* * `grant` is the name of the privilege, such as `Resource`.
* * `args` provides a list of parameters that the function requesting the privilege calls.
*
*/
@Experimental()
type SecurityManager = (grant: String, args: Array<Any>) -> Boolean

/***
* Service that handles all logging:
*
* * `initialize`:
*    Function called when the execution starts. DataWeave sends
*    the result to every `log` call through the `context` parameter,
*    so that, for example, a logging header can be sent at
*     initialization and recovered in each log.
* * `log`:
*    Function that is called on every log message.
* * `shutdown`:
*    Function called when the execution completes, which is a common time
*    to flush any buffer or to log out gracefully.
*/
@Experimental()
type LoggerService = {
    initialize?: () -> {},
    log: (level: LogLevel, msg: String, context: {}) -> Any,
    shutdown?: () -> Boolean
}

/**
* Configuration of the runtime execution that has advanced parameters.
*
* * `timeOut`:
*    Maximum amount of time the DataWeave script takes before timing out.
*
* * `outputMimeType`:
*   Default output MIME type if not specified in the DataWeave script.
*
* * `writerProperties`:
*    Writer properties to use with the specified the `outputMimeType` property.
*
* * `onException`
*   Specifies the behavior that occurs when the execution fails:
*   ** `HANDLE` (default value) returns `ExecutionFailure`.
*   ** `FAIL` propagates an exception.
*
* * `securityManager`:
*   Identifies the `SecurityManager` to use in this execution. This security manager
*   is composed by the current `SecurityManager`.
*
* * `loggerService`:
*   The `LoggerService` to use in this execution.
* * `maxStackSize`:
*   The maximum stack size.
*
* * `onUnhandledTimeout`:
*  Callback that is called when the watchdog was not able to stop the execution
*  after a timeout, which is useful for logging or reporting the problem.
*  The callback is called with the following:
* ** `threadName`:  Name of the thread that hanged.
* ** `javaStackTrace`: Java stack trace where the hang occurred.
* ** `code`: The DataWeave code that caused the hang.
*/
@Experimental()
type RuntimeExecutionConfiguration = {
    timeOut?: Number,
    outputMimeType?: MimeType,
    writerProperties?: Dictionary<SimpleType>,
    onException?: "HANDLE" | "FAIL",
    securityManager?: SecurityManager,
    loggerService?: LoggerService,
    maxStackSize?:Number,
    onUnhandledTimeout?: (threadName: String, javaStackTrace: String, code: String) -> Any
}

/**
* Runs the input script under the provided context and executes
* the script in the current runtime.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | fileToExecute | Name of the file to execute.
* | fs | File system that contains the file to execute.
* | readerInput | Inputs to read and bind to the execution.
* | inputValues | Inputs to bind directly to the execution.
* | configuration | The runtime configuration.
* |===
*
* === Example
*
* This example shows how `run` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* import * from dw::Runtime
* var jsonValue = {
*   value: '{"name": "Mariano"}' as Binary {encoding: "UTF-8"},
*   encoding: "UTF-8",
*   properties: {},
*   mimeType: "application/json"
* }
*
* var jsonValue2 = {
*   value: '{"name": "Mariano", "lastName": "achaval"}' as Binary {encoding: "UTF-8"},
*   encoding: "UTF-8",
*   properties: {},
*   mimeType: "application/json"
* }
*
* var invalidJsonValue = {
*   value: '{"name": "Mariano' as Binary {encoding: "UTF-8"},
*   encoding: "UTF-8",
*   properties: {},
*   mimeType: "application/json"
* }
*
* var Utils = "fun sum(a,b) = a +b"
* ---
* {
*   "execute_ok" : run("main.dwl", {"main.dwl": "{a: 1}"}, {"payload": jsonValue }),
*   "logs" : do {
*     var execResult = run("main.dwl", {"main.dwl": "{a: log(1)}"}, {"payload": jsonValue })
*     ---
*     {
*         m: execResult.logs.message,
*         l: execResult.logs.level
*     }
*   },
*   "grant" : run("main.dwl", {"main.dwl": "{a: readUrl(`http://google.com`)}"}, {"payload": jsonValue }, { securityManager: (grant, args) -> false }),
*   "library" : run("main.dwl", {"main.dwl": "Utils::sum(1,2)", "/Utils.dwl": Utils }, {"payload": jsonValue }),
*   "timeout" : run("main.dwl", {"main.dwl": "(1 to 1000000000000) map \$ + 1" }, {"payload": jsonValue }, {timeOut: 2}).success,
*   "execFail" : run("main.dwl", {"main.dwl": "dw::Runtime::fail('My Bad')" }, {"payload": jsonValue }),
*   "parseFail" : run("main.dwl", {"main.dwl": "(1 + " }, {"payload": jsonValue }),
*   "writerFail" : run("main.dwl", {"main.dwl": "output application/xml --- 2" }, {"payload": jsonValue }),
*   "readerFail" : run("main.dwl", {"main.dwl": "output application/xml --- payload" }, {"payload": invalidJsonValue }),
*   "defaultOutput" : run("main.dwl", {"main.dwl": "payload" }, {"payload": jsonValue2}, {outputMimeType: "application/csv", writerProperties: {"separator": "|"}}),
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "execute_ok": {
*     "success": true,
*     "value": "{\n  a: 1\n}",
*     "mimeType": "application/dw",
*     "encoding": "UTF-8",
*     "logs": [
*
*     ]
*   },
*   "logs": {
*     "m": [
*       "1"
*     ],
*     "l": [
*       "INFO"
*     ]
*   },
*   "grant": {
*     "success": false,
*     "message": "The given required permissions: `Resource` are not being granted for this execution.\nTrace:\n  at readUrl (Unknown)\n  at main::main (line: 1, column: 5)",
*     "location": {
*       "start": {
*         "index": 0,
*         "line": 0,
*         "column": 0
*       },
*       "end": {
*         "index": 0,
*         "line": 0,
*         "column": 0
*       },
*       "content": "Unknown location"
*     },
*     "stack": [
*       "readUrl (anonymous:0:0)",
*       "main (main:1:5)"
*     ],
*     "logs": [
*
*     ]
*   },
*   "library": {
*     "success": true,
*     "value": "3",
*     "mimeType": "application/dw",
*     "encoding": "UTF-8",
*     "logs": [
*
*     ]
*   },
*   "timeout": false,
*   "execFail": {
*     "success": false,
*     "message": "My Bad\nTrace:\n  at fail (Unknown)\n  at main::main (line: 1, column: 1)",
*     "location": {
*       "start": {
*         "index": 0,
*         "line": 0,
*         "column": 0
*       },
*       "end": {
*         "index": 0,
*         "line": 0,
*         "column": 0
*       },
*       "content": "Unknown location"
*     },
*     "stack": [
*       "fail (anonymous:0:0)",
*       "main (main:1:1)"
*     ],
*     "logs": [
*
*     ]
*   },
*   "parseFail": {
*     "success": false,
*     "message": "Invalid input \"1 + \", expected parameter or parenEnd (line 1, column 2):\n\n\n1| (1 + \n    ^^^^\nLocation:\nmain (line: 1, column:2)",
*     "location": {
*       "start": {
*         "index": 0,
*         "line": 1,
*         "column": 2
*       },
*       "end": {
*         "index": 4,
*         "line": 1,
*         "column": 6
*       },
*       "content": "\n1| (1 + \n    ^^^^"
*     },
*     "logs": [
*
*     ]
*   },
*   "writerFail": {
*     "success": false,
*     "message": "Trying to output non-whitespace characters outside main element tree (in prolog or epilog), while writing Xml at .",
*     "location": {
*       "content": ""
*     },
*     "stack": [
*
*     ],
*     "logs": [
*
*     ]
*   },
*   "readerFail": {
*     "success": false,
*     "message": "Unexpected end-of-input at payload@[1:18] (line:column), expected '\"', while reading `payload` as Json.\n \n1| {\"name\": \"Mariano\n                    ^",
*     "location": {
*       "content": "\n1| {\"name\": \"Mariano\n                    ^"
*     },
*     "stack": [
*
*     ],
*     "logs": [
*
*     ]
*   },
*   "defaultOutput": {
*     "success": true,
*     "value": "name|lastName\nMariano|achaval\n",
*     "mimeType": "application/csv",
*     "encoding": "UTF-8",
*     "logs": [
*
*     ]
*   }
* }
* ----
**/
@Experimental()
@RuntimePrivilege(requires = "Execution")
fun run(fileToExecute: String, fs: Dictionary<String>, readerInputs: Dictionary<ReaderInput> = {}, inputValues: Dictionary<Any> = {}, configuration: RuntimeExecutionConfiguration = {}): RunResult =
      native("system::RunScriptFunctionValue")

/**
* Evaluates a script with the specified context and returns the result of that evaluation.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | fileToExecute | Name of the file to execute.
* | fs | An object that contains the file to evaluate.
* | readerInputs | Reader inputs to bind.
* | inputValues | Additional literal values to bind
* | configuration | The runtime configuration.
* |===
*
* === Example
*
* This example shows how `eval` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::Runtime
*
* var jsonValue = {
*    value: '{"name": "Mariano"}' as Binary {encoding: "UTF-8"},
*    encoding: "UTF-8",
*    properties: {},
*    mimeType: "application/json"
* }
*
* var jsonValue2 = {
*    value: '{"name": "Mariano", "lastName": "achaval"}' as Binary {encoding: "UTF-8"},
*    encoding: "UTF-8",
*    properties: {},
*    mimeType: "application/json"
* }
*
* var invalidJsonValue = {
*    value: '{"name": "Mariano' as Binary {encoding: "UTF-8"},
*    encoding: "UTF-8",
*    properties: {},
*    mimeType: "application/json"
* }
*
* var Utils = "fun sum(a,b) = a +b"
* output application/json
* ---
* {
*    "execute_ok" : run("main.dwl", {"main.dwl": "{a: 1}"}, {"payload": jsonValue }),
*    "logs" : do {
*      var execResult = run("main.dwl", {"main.dwl": "{a: log(1)}"}, {"payload": jsonValue })
*      ---
*      {
*          m: execResult.logs.message,
*          l: execResult.logs.level
*      }
*    },
*    "grant" : eval("main.dwl", {"main.dwl": "{a: readUrl(`http://google.com`)}"}, {"payload": jsonValue }, {},{ securityManager: (grant, args) -> false }),
*    "library" : eval("main.dwl", {"main.dwl": "Utils::sum(1,2)", "/Utils.dwl": Utils }, {"payload": jsonValue }),
*    "timeout" : eval("main.dwl", {"main.dwl": "(1 to 1000000000000) map \$ + 1" }, {"payload": jsonValue }, {},{timeOut: 2}).success,
*    "execFail" : eval("main.dwl", {"main.dwl": "dw::Runtime::fail('My Bad')" }, {"payload": jsonValue }),
*    "parseFail" : eval("main.dwl", {"main.dwl": "(1 + " }, {"payload": jsonValue }),
*    "writerFail" : eval("main.dwl", {"main.dwl": "output application/xml --- 2" }, {"payload": jsonValue }),
*    "defaultOutput" : eval("main.dwl", {"main.dwl": "payload" }, {"payload": jsonValue2}, {},{outputMimeType: "application/csv", writerProperties: {"separator": "|"}}),
*    "onExceptionFail": do  {
*      dw::Runtime::try( () ->
*          eval("main.dwl", {"main.dwl": "dw::Runtime::fail('Failing Test')" }, {"payload": jsonValue2}, {},{onException: "FAIL"})
*      ).success
*    },
*    "customLogger":
*         eval(
*   "main.dwl",
*            {"main.dwl": "log(1234)" },
*    {"payload": jsonValue2},
*     {},
*    {
*                   loggerService: {
*                      initialize: () -> {token: "123"},
*                      log: (level, msg, context) -> log("$(level) $(msg)", context)
*                   }
*                 }
*            )
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "execute_ok": {
*     "success": true,
*     "value": "{\n  a: 1\n}",
*     "mimeType": "application/dw",
*     "encoding": "UTF-8",
*     "logs": [
*
*     ]
*   },
*   "logs": {
*     "m": [
*       "1"
*     ],
*     "l": [
*       "INFO"
*     ]
*   },
*   "grant": {
*     "success": false,
*     "message": "The given required permissions: `Resource` are not being granted for this execution.\nTrace:\n  at readUrl (Unknown)\n  at main::main (line: 1, column: 5)",
*     "location": {
*       "start": {
*         "index": 0,
*         "line": 0,
*         "column": 0
*       },
*       "end": {
*         "index": 0,
*         "line": 0,
*         "column": 0
*       },
*       "content": "Unknown location"
*     },
*     "stack": [
*       "readUrl (anonymous:0:0)",
*       "main (main:1:5)"
*     ],
*     "logs": [
*
*     ]
*   },
*   "library": {
*     "success": true,
*     "value": 3,
*     "logs": [
*
*     ]
*   },
*   "timeout": true,
*   "execFail": {
*     "success": false,
*     "message": "My Bad\nTrace:\n  at fail (Unknown)\n  at main::main (line: 1, column: 1)",
*     "location": {
*       "start": {
*         "index": 0,
*         "line": 0,
*         "column": 0
*       },
*       "end": {
*         "index": 0,
*         "line": 0,
*         "column": 0
*       },
*       "content": "Unknown location"
*     },
*     "stack": [
*       "fail (anonymous:0:0)",
*       "main (main:1:1)"
*     ],
*     "logs": [
*
*     ]
*   },
*   "parseFail": {
*     "success": false,
*     "message": "Invalid input \"1 + \", expected parameter or parenEnd (line 1, column 2):\n\n\n1| (1 + \n    ^^^^\nLocation:\nmain (line: 1, column:2)",
*     "location": {
*       "start": {
*         "index": 0,
*         "line": 1,
*         "column": 2
*       },
*       "end": {
*         "index": 4,
*         "line": 1,
*         "column": 6
*       },
*       "content": "\n1| (1 + \n    ^^^^"
*     },
*     "logs": [
*
*     ]
*   },
*   "writerFail": {
*     "success": true,
*     "value": 2,
*     "logs": [
*
*     ]
*   },
*   "defaultOutput": {
*     "success": true,
*     "value": {
*       "name": "Mariano",
*       "lastName": "achaval"
*     },
*     "logs": [
*
*     ]
*   },
*   "onExceptionFail": false,
*   "customLogger": {
*     "success": true,
*     "value": 1234,
*     "logs": [
*
*     ]
*   }
* }
* ----
**/
@Experimental()
@RuntimePrivilege(requires = "Execution")
fun eval(fileToExecute: String, fs: Dictionary<String>, readerInputs: Dictionary<ReaderInput> = {}, inputValues: Dictionary<Any> = {}, configuration: RuntimeExecutionConfiguration = {}): EvalResult =
      native("system::EvalScriptFunctionValue")

/**
* Runs the script at the specified URL.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | url | The name of the file to execute.
* | readerInputs | Inputs to read and bind to the execution.
* | inputValues | Inputs to be bind directly to the execution.
* | configuration | The runtime configuration.
* |===
*
* === Example
*
* This example shows how `runUrl` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* import * from dw::Runtime
* var jsonValue = {
*   value: '{"name": "Mariano"}' as Binary {encoding: "UTF-8"},
*   encoding: "UTF-8",
*   properties: {},
*   mimeType: "application/json"
* }
*
* var Utils = "fun sum(a,b) = a +b"
* ---
* {
*   "execute_ok" : runUrl("classpath://org/mule/weave/v2/engine/runtime_runUrl/example.dwl", {"payload": jsonValue })
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "execute_ok": {
*      "success": true,
*      "value": "\"Mariano\"",
*      "mimeType": "application/dw",
*      "encoding": "UTF-8",
*      "logs": [
*
*      ]
*    }
*  }
* ----
**/
@Experimental()
@RuntimePrivilege(requires = "Execution")
fun runUrl(url: String, readerInputs: Dictionary<ReaderInput> = {}, inputValues: Dictionary<Any> = {}, configuration: RuntimeExecutionConfiguration = {}): RunResult = do {
    run("Main.dwl",{"Main.dwl": readUrl(url, "text/plain") as String {encoding: "UTF-8"}}, readerInputs , inputValues,configuration)
}

/**
* Evaluates the script at the specified URL.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | url | Name of the file execute.
* | readerInputs | Inputs to read and bind to the execution.
* | inputValues | Inputs to bind directly to the execution.
* | configuration | The runtime configuration.
* |===
*
* === Example
*
* This example shows how `evalUrl` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::Runtime
* var jsonValue = {
*   value: '{"name": "Mariano"}' as Binary {encoding: "UTF-8"},
*   encoding: "UTF-8",
*   properties: {},
*   mimeType: "application/json"
* }
*
* var Utils = "fun sum(a,b) = a +b"
* output application/json
* ---
* {
*   "execute_ok" : evalUrl("classpath://org/mule/weave/v2/engine/runtime_evalUrl/example.dwl", {"payload": jsonValue }),
*   "execute_ok_withValue" : evalUrl("classpath://org/mule/weave/v2/engine/runtime_evalUrl/example.dwl", {}, {"payload": {name: "Mariano"}})
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "execute_ok": {
*      "success": true,
*      "value": "Mariano",
*      "logs": [
*
*      ]
*    },
*    "execute_ok_withValue": {
*      "success": true,
*      "value": "Mariano",
*      "logs": [
*
*      ]
*    }
*  }
* ----
**/
@Experimental()
@RuntimePrivilege(requires = "Execution")
fun evalUrl(url: String, readerInputs: Dictionary<ReaderInput> = {}, inputValues: Dictionary<Any> = {}, configuration: RuntimeExecutionConfiguration = {}): EvalResult = do {
    eval("Main.dwl",{"Main.dwl": readUrl(url, "text/plain") as String {encoding: "UTF-8"}}, readerInputs , inputValues,configuration)
}

/**
* Returns an array of all `DataFormatDescriptor` values that are installed in
* the current instance of DataWeave.
*
* === Example
*
* This example shows how `dataFormatsDescriptor` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* import * from dw::Runtime
* ---
* dataFormatsDescriptor()
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* [
*       {
*            "id": "json",
*            "binary": false,
*            "defaultEncoding": "UTF-8",
*            "extensions": [
*              ".json"
*            ],
*            "defaultMimeType": "application/json",
*            "acceptedMimeTypes": [
*              "application/json"
*            ],
*            "readerProperties": [
*              {
*                "name": "streaming",
*                "optional": true,
*                "defaultValue": false,
*                "description": "Used for streaming input (use only if entries are accessed sequentially).",
*                "possibleValues": [
*                  true,
*                  false
*                ]
*              }
*            ],
*            "writerProperties": [
*              {
*                "name": "writeAttributes",
*                "optional": true,
*                "defaultValue": false,
*                "description": "Indicates that if a key has attributes, they are going to be added as children key-value pairs of the key that contains them. The attribute new key name will start with @.",
*                "possibleValues": [
*                  true,
*                  false
*                ]
*              },
*              {
*               "name": "skipNullOn",
*                  "optional": true,
*                  "defaultValue": "None",
*                  "description": "Indicates where is should skips null values if any or not. By default it doesn't skip.",
*                  "possibleValues": [
*                    "arrays",
*                    "objects",
*                    "everywhere"
*                  ]
*                }
*              ]
*            },
*            {
*              "id": "xml",
*              "binary": false,
*              "extensions": [
*                ".xml"
*              ],
*              "defaultMimeType": "application/xml",
*              "acceptedMimeTypes": [
*                "application/xml"
*              ],
*              "readerProperties": [
*                {
*                "name": "supportDtd",
*                "optional": true,
*                "defaultValue": true,
*                "description": "Whether DTD handling is enabled or disabled; disabling means both internal and external subsets will just be skipped unprocessed.",
*                "possibleValues": [
*                  true,
*                  false
*                ]
*              },
*              {
*                "name": "streaming",
*                "optional": true,
*                "defaultValue": false,
*                "description": "Used for streaming input (use only if entries are accessed sequentially).",
*                "possibleValues": [
*                  true,
*                  false
*                ]
*              },
*              {
*                "name": "maxEntityCount",
*                "optional": true,
*                "defaultValue": 1,
*                "description": "The maximum number of entity expansions. The limit is in place to avoid Billion Laughs attacks.",
*                "possibleValues": [
*
*                ]
*              }
*            ],
*            "writerProperties": [
*              {
*                "name": "writeDeclaration",
*                "optional": true,
*                "defaultValue": true,
*                "description": "Indicates whether to write the XML header declaration or not.",
*                "possibleValues": [
*                  true,
*                  false
*                ]
*              },
*              {
*                "name": "indent",
*                "optional": true,
*                "defaultValue": true,
*                "description": "Indicates whether to indent the code for better readability or to compress it into a single line.",
*                "possibleValues": [
*                  true,
*                  false
*                ]
*              }
*            ]
*          }
* ]
* ----
**/
@Experimental()
fun dataFormatsDescriptor(): Array<DataFormatDescriptor> = native("system::DataFormatDescriptorsFunctionValue")

/**
 * Stops the execution for the specified timeout period (in milliseconds).
 *
 *
 * WARNING: Stopping the execution blocks the thread, potentially
 * causing slowness, low performance and potentially freezing of the entire
 * runtime. This operation is intended for limited functional testing purposes.
 * Do not use this function in a production application, performance testing, or
 * with multiple applications deployed.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | value | Input of any type.
 * | timeout | The number of milliseconds to wait.
 * |===
 *
 * === Example
 *
 * This example waits 2000 milliseconds (2 seconds) to execute.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import * from dw::Runtime
 * output application/json
 * ---
 * { "user" : 1 } wait 2000
 * ----
 *
 * ==== Output
 *
 * [source,JSON,linenums]
 * ----
 * { "user": 1 }
 * ----
 */
fun wait <T>(value: T, timeout: Number): T = native("system::wait")


// A type: `TryResult`.
/**
 * Object with a result or error message. If `success` is `false`, data type provides
 * the `error`. If `true`, the data type provides the `result`.
 */
type TryResult<T> = Result<T, TryResultFailure>

/**
 * A type for representing failed execution from `try`.
 *
 * Supports the following fields:
 *
 * * `kind`: The error kind.
 * * `message`: The error message.
 * * `stack`: The stacktrace error (optional).
 * * `stackTrace`: The stacktrace string value representation (optional).
 * * `location`: The error location (optional).
 *
 * Starting in Mule 4.4.0, if the stack is not present, the `stackTrace` field is available
 * with the native Java stack trace.
 */
@Since(version = "2.7.0")
type TryResultFailure = {
  kind: String,
  message: String,
  stack?: Array<String>,
  stackTrace?: String,
  location?: String
}

/**
 * Evaluates the delegate function and returns an object with `success: true` and `result` if the delegate function succeeds, or an object with `success: false` and `error` if the delegate function throws an exception.
 *
 *
 * The `orElseTry` and `orElse` functions will also continue processing if the `try` function fails. See the `orElseTry` and `orElse` documentation for more complete examples of handling failing `try` function expressions.
 *
 *
 * Note: Instead of using the `orElseTry` and `orElse` functions, based on the output of the `try` function, you can add conditional logic to execute when the result is `success: true` or `success: false`.
 *
 * === Parameters
 *
 * [%header, cols="1,3"]
 * |===
 * | Name | Description
 * | delegate | The function to evaluate.
 * |===
 *
 * === Example
 *
 * This example calls the `try` function using the `randomNumber` function as argument.
 * The function `randomNumber` generates a random number and calls `fail` if the number is greater than 0.5. The declaration of this function is in the script's header.
 *
 * ==== Source
 *
 * [source,DataWeave,linenums]
 * ----
 * %dw 2.0
 * import try, fail from dw::Runtime
 * output application/json
 * fun randomNumber() =
 * if(random() > 0.5)
 *   fail("This function is failing")
 *  else
 *   "OK"
 * ---
 * try(() -> randomNumber())
 * ----
 *
 * ==== Output
 *
 * When `randomNumber` fails, the output is:
 *
 * [source,JSON,linenums]
 * ----
 * {
 *   "success": false,
 *   "error": {
 *     "kind": "UserException",
 *     "message": "This function is failing",
 *     "location": "Unknown location",
 *     "stack": [
 *       "fail (anonymous:0:0)",
 *       "myFunction (anonymous:1:114)",
 *       "main (anonymous:1:179)"
 *     ]
 *   }
 * }
 * ----
 *
 * When `randomNumber` succeeds, the output is:
 *
 * [source,JSON,linenums]
 * ----
 * {
 *   "success": true,
 *   "result": "OK"
 * }
 * ----
 */
fun try<T>(delegate: () -> T): TryResult<T> = native("system::try")

/**
* Function to use with `try` to chain multiple `try` requests.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | previous | Result from a previous call to `try`.
* | orElseTry | Argument to try if the `previous` argument fails.
* |===
*
* === Example
*
* This example waits shows how to chain different try
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::Runtime
* var user = {}
* var otherUser = {}
* output application/json
* ---
* {
*     a: try(() -> user.name!) orElseTry otherUser.name!,
*     b: try(() -> user.name!) orElseTry "No User Name"
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "a": {
*     "success": false,
*     "error": {
*       "kind": "KeyNotFoundException",
*       "message": "There is no key named 'name'",
*       "location": "\n9|     a: try(() -> user.name!) orElseTry otherUser.name!,\n                                          ^^^^^^^^^^^^^^",
*       "stack": [
*         "main (org::mule::weave::v2::engine::transform:9:40)"
*       ]
*     }
*   },
*   "b": {
*     "success": true,
*     "result": "No User Name"
*   }
* }
* ----
*/
@Since(version = "2.2.0")
fun orElseTry<T, R>(previous: TryResult<T>, orElse: () -> R): TryResult<T | R> = do {
    if(previous.success)
       previous
    else
       try(orElse)
}

/**
* Returns the result of the `orElse` argument if the `previous` argument to
* `try` fails. Otherwise, the function returns the value of the `previous`
* argument.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | previous | Result from a previous call to `try`.
* | orElse | Argument to return if the `previous` argument fails.
* |===
*
* === Example
*
* This example waits shows how to chain different try
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::Runtime
* var user = {}
* var otherUser = {name: "DW"}
* output application/json
* ---
* {
*     a: try(() -> user.name!) orElse "No User Name",
*     b: try(() -> otherUser.name) orElse "No User Name"
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "a": "No User Name",
*   "b": "DW"
* }
* ----
*/
@Since(version = "2.2.0")
fun orElse<T, E, R>(previous: Result<T, E>, orElse: () -> R): T | R = do {
    if(previous.success)
       previous.result!
    else
       orElse()
}

/**
* Returns the location string of a given value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | value | A value of any type.
* |===
*
* === Example
*
* This example returns the contents of the line (the location) that defines
* variable `a` in the header of the DataWeave script.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::Runtime
* var a = 123
* output application/json
* ---
* locationString(a)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* "var a = 123"
* ----
*/
fun locationString(value:Any): String = do {
  var loc = location(value)
  ---
  loc.text default loc.locationString
}

/**
* Returns the location of a given value, or `null` if the
* location can't be traced back to a DataWeave file.
*
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | value | A value of any type.
* |===
*
* === Example
*
* This example returns the location that defines
* the function `sqrt` in the `dw::Core` module.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import location from dw::Runtime
* output application/json
* ---
* location(sqrt)
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "uri": "/dw/Core.dwl",
*   "nameIdentifier": "dw::Core",
*   "startLine": 5797,
*   "startColumn": 36,
*   "endLine": 5797,
*   "endColumn": 77
* }
* ----
*/
@Since(version = "2.4.0")
fun location(value: Any): Location = native("system::location")

/**
* Returns all the properties configured for the DataWeave runtime, which executes the language.
*
* === Example
*
* This example returns all properties from the `java.util.Properties` class.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::Runtime
* output application/dw
* ---
* { "props" : props() }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*  props: {
*    "java.vendor": "Oracle Corporation" as String {class: "java.lang.String"},
*    "sun.java.launcher": "SUN_STANDARD" as String {class: "java.lang.String"},
*    "sun.management.compiler": "HotSpot 64-Bit Tiered Compilers" as String ..., *    "os.name": "Mac OS X" as String {class: "java.lang.String"},
*    "sun.boot.class.path": "/Library/Java/JavaVirtualMachines/ ...,
*    "org.glassfish.grizzly.nio.transport.TCPNIOTransport...": "1048576" ...,
*    "java.vm.specification.vendor": "Oracle Corporation" as String ...,
*    "java.runtime.version": "1.8.0_111-b14" as String {class: "java.lang.String"},
*    "wrapper.native_library": "wrapper" as String {class: "java.lang.String"},
*    "wrapper.key": "XlIl4YartmfEU3oKu7o81kNQbwhveXi-" as String ...,
*    "user.name": "me" as String {class: "java.lang.String"},
*    "mvel2.disable.jit": "TRUE" as String {class: "java.lang.String"},
*    "user.language": "en" as String {class: "java.lang.String"} ...,
*    "sun.boot.library.path": "/Library/Java/JavaVirtualMachines ...
*    "xpath.provider": "com.mulesoft.licm.DefaultXPathProvider" ...,
*    "wrapper.backend": "pipe" as String {class: "java.lang.String"},
*    "java.version": "1.8.0_111" as String {class: "java.lang.String"},
*    "user.timezone": "America/Los_Angeles" as String {class: "java.lang.String"},
*    "java.net.preferIPv4Stack": "TRUE" as String {class: "java.lang.String"},
*    "sun.arch.data.model": "64" as String {class: "java.lang.String"},
*    "java.endorsed.dirs": "/Library/Java/JavaVirtualMachines/...,
*    "sun.cpu.isalist": "" as String {class: "java.lang.String"},
*    "sun.jnu.encoding": "UTF-8" as String {class: "java.lang.String"},
*    "mule.testingMode": "" as String {class: "java.lang.String"},
*    "file.encoding.pkg": "sun.io" as String {class: "java.lang.String"},
*    "file.separator": "/" as String {class: "java.lang.String"},
*    "java.specification.name": "Java Platform API Specification" ...,
*    "java.class.version": "52.0" as String {class: "java.lang.String"},
*    "jetty.git.hash": "82b8fb23f757335bb3329d540ce37a2a2615f0a8" ...,
*    "user.country": "US" as String {class: "java.lang.String"},
*    "mule.agent.configuration.folder": "/Applications/AnypointStudio.app/ ...,
*    "log4j.configurationFactory": "org.apache.logging.log4j.core...",
*    "java.home": "/Library/Java/JavaVirtualMachines/...,
*    "java.vm.info": "mixed mode" as String {class: "java.lang.String"},
*    "wrapper.version": "3.5.34-st" as String {class: "java.lang.String"},
*    "os.version": "10.13.4" as String {class: "java.lang.String"},
*    "org.eclipse.jetty.LEVEL": "WARN" as String {class: "java.lang.String"},
*    "path.separator": ":" as String {class: "java.lang.String"},
*    "java.vm.version": "25.111-b14" as String {class: "java.lang.String"},
*    "wrapper.pid": "5212" as String {class: "java.lang.String"},
*    "java.util.prefs.PreferencesFactory": "com.mulesoft.licm..."},
*    "wrapper.java.pid": "5213" as String {class: "java.lang.String"},
*    "mule.home": "/Applications/AnypointStudio.app/...,
*    "java.awt.printerjob": "sun.lwawt.macosx.CPrinterJob" ...,
*    "sun.io.unicode.encoding": "UnicodeBig" as String {class: "java.lang.String"},
*    "awt.toolkit": "sun.lwawt.macosx.LWCToolkit" ...,
*    "org.glassfish.grizzly.nio.transport...": "1048576" ...,
*    "user.home": "/Users/me" as String {class: "java.lang.String"},
*    "java.specification.vendor": "Oracle Corporation" ...,
*    "java.library.path": "/Applications/AnypointStudio.app/...,
*    "java.vendor.url": "http://java.oracle.com/" as String ...,
*    "java.vm.vendor": "Oracle Corporation" as String {class: "java.lang.String"},
*    gopherProxySet: "false" as String {class: "java.lang.String"},
*    "wrapper.jvmid": "1" as String {class: "java.lang.String"},
*    "java.runtime.name": "Java(TM) SE Runtime Environment" ...,
*    "mule.encoding": "UTF-8" as String {class: "java.lang.String"},
*    "sun.java.command": "org.mule.runtime.module.reboot....",
*    "java.class.path": "%MULE_LIB%:/Applications/AnypointStudio.app...",
*    "log4j2.loggerContextFactory": "org.mule.runtime.module.launcher...,
*    "java.vm.specification.name": "Java Virtual Machine Specification" ,
*    "java.vm.specification.version": "1.8" as String {class: "java.lang.String"},
*    "sun.cpu.endian": "little" as String {class: "java.lang.String"},
*    "sun.os.patch.level": "unknown" as String {class: "java.lang.String"},
*    "com.ning.http.client.AsyncHttpClientConfig.useProxyProperties": "true" ...,
*    "wrapper.cpu.timeout": "10" as String {class: "java.lang.String"},
*    "java.io.tmpdir": "/var/folders/42/dd73l3rx7qz0n625hr29kty80000gn/T/" ...,
*    "anypoint.platform.analytics_base_uri": ...,
*    "java.vendor.url.bug": "http://bugreport.sun.com/bugreport/" ...,
*    "os.arch": "x86_64" as String {class: "java.lang.String"},
*    "java.awt.graphicsenv": "sun.awt.CGraphicsEnvironment" ...,
*    "mule.base": "/Applications/AnypointStudio.app...",
*    "java.ext.dirs": "/Users/staceyduke/Library/Java/Extensions: ..."},
*    "user.dir": "/Applications/AnypointStudio.app/..."},
*    "line.separator": "\n" as String {class: "java.lang.String"},
*    "java.vm.name": "Java HotSpot(TM) 64-Bit Server VM" ...,
*    "org.quartz.scheduler.skipUpdateCheck": "true" ...,
*    "file.encoding": "UTF-8" as String {class: "java.lang.String"},
*    "mule.forceConsoleLog": "" as String {class: "java.lang.String"},
*    "java.specification.version": "1.8" as String {class: "java.lang.String"},
*    "wrapper.arch": "universal" as String {class: "java.lang.String"}
*  } as Object {class: "java.util.Properties"}
* ----
*/
@RuntimePrivilege(requires = "Properties")
fun props(): Dictionary<String> = native("system::props")

/**
* Returns the value of the property with the specified name or `null` if the
* property is not defined.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | propertyName | The property to retrieve.
* |===
*
* === Example
*
* This example gets the `user.timezone` property.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::Runtime
* output application/dw
* ---
* { "props" : prop("user.timezone") }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { props: "America/Los_Angeles" as String {class: "java.lang.String"} }
* ----
*/
@RuntimePrivilege(requires = "Properties")
fun prop(propertyName: String): String | Null = props()[propertyName]

/**
*
* Returns the DataWeave version that is currently running.
*
* === Example
*
* This example returns the DataWeave version.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::Runtime
* output application/json
* ---
* version()
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* "2.5"
* ----
**/
@Since(version = "2.5.0")
fun version(): String = native("system::version")

/**
* Returns the `DataFormatDescriptor` based on the `dw::module::Mime::MimeType` specified or `null` if
* there is no `DataFormatDescriptor` for the given `MimeType`.
*
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | mimeType | `dw::module::Mime::MimeType` | The MIME type value to search.
* |===
*
* === Example
*
* This example searches for a JSON `DataFormatDescriptor` and an unknown `DataFormatDescriptor`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::Runtime
* output application/json
*
* var jsonDF = findDataFormatDescriptorByMime({'type': "*", subtype: "json", parameters: {}})
* var unknownDF = findDataFormatDescriptorByMime({'type': "*", subtype: "*", parameters: {}})
*
* fun simplify(df: DataFormatDescriptor | Null) = df  match {
*   case d is DataFormatDescriptor -> { name: d.name, defaultMimeType: d.defaultMimeType }
*   case is Null -> { name: "unknown", defaultMimeType: "unknown" }
* }
* ---
* {
*   json: simplify(jsonDF),
*   unknown: simplify(unknownDF)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "json": {
*     "name": "json",
*     "defaultMimeType": "application/json"
*   },
*   "unknown": {
*     "name": "unknown",
*     "defaultMimeType": "unknown"
*   }
* }
* ----
*
**/
@Since(version = "2.7.0")
@Experimental()
fun findDataFormatDescriptorByMime(mime: dw::module::Mime::MimeType): DataFormatDescriptor | Null = native("system::FindDataFormatDescriptorByMimeFunctionValue")
