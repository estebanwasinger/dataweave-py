/**
 * This helper module provide functions for creating MultiPart and
 * formats and parts (including fields and boundaries) of MultiPart formats.
 *
 *
 * To use this module, you must import it into your DataWeave code, for example,
 * by adding the line `import dw::module::Multipart` to the header of your
 * DataWeave script.
 */
%dw 2.0

/**
* `MultipartPart` type, a data structure for a part within a `MultiPart` format.
* See the output examples for the Multipart `field` function
* https://docs.mulesoft.com/dataweave/latest/dw-multipart-functions-field[documentation].
*/
type MultipartPart = {
  headers?: {
    "Content-Disposition"?: {
      name: String,
      filename?: String
    },
    "Content-Type"?: String
  },
  content: Any
}

/**
* `MultiPart` type, a data structure for a complete `Multipart` format. See the
* output example for the Multipart `form` function
* https://docs.mulesoft.com/dataweave/latest/dw-multipart-functions-form[documentation].
*/
type Multipart = {
  preamble?: String,
  parts: {
    _?: MultipartPart
  }
}

/**
* Creates a `MultipartPart` data structure using the specified part name,
* input content for the part, format (or mime type), and optionally, file name.
*
*
* This version of the `field` function accepts arguments as an array of objects
* that use the parameter names as keys, for example:
* `Multipart::field({name:"order",value: myOrder, mime: "application/json", fileName: "order.json"})`
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name | Description
* | opts | Array of objects that specifies:
*
* * A unique `name` (required) for the `Content-Disposition` header of the part.
* * A `value` (required) for the content of the part.
* * `mime` (optional for strings) for the mime type (for example, `application/json`) to apply to content within the part. This setting can be used to transform the input type, for example, from JSON to XML.
* * An optional `fileName` value that you can supply to the `filename` parameter in the part's `Content-Disposition` header.
* |===
*
* === Example
*
* This example creates a `Multipart` data structure that contains parts.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::module::Multipart
* output multipart/form-data
* var firstPart = "content for my first part"
* var secondPart = "content for my second part"
* ---
* {
*   parts: {
*     part1: Multipart::field({name:"myFirstPart",value: firstPart}),
*     part2: Multipart::field("mySecondPart", secondPart)
*   }
* }
* ----
*
* ==== Output
*
* [source,json,linenums]
* ----
* ------=_Part_320_1528378161.1542639222352
* Content-Disposition: form-data; name="myFirstPart"
* content for my first part
* ------=_Part_320_1528378161.1542639222352
* Content-Disposition: form-data; name="mySecondPart"
* content for my second part
* ------=_Part_320_1528378161.1542639222352--
* ----
*
* === Example
*
* This example produces two parts. The first part (named `order`) outputs
* content in JSON and provides a file name for the part (`order.json`). The
* second (named `clients`) outputs content in XML and does not provide a file
* name. Also notice that in this example you need to add the function's
* namespace to the function name, for example, `Multipart::field`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::module::Multipart
* output multipart/form-data
* var myOrder = [
*   {
*     order: 1,
*     amount: 2
*   },
*   {
*     order: 32,
*     amount: 1
*   }
* ]
* var myClients = {
*   clients: {
*     client: {
*       id: 1,
*       name: "Mariano"
*     },
*     client: {
*       id: 2,
*       name: "Shoki"
*     }
*   }
* }
* ---
* {
*   parts: {
*     order: Multipart::field({name:"order",value: myOrder, mime: "application/json", fileName: "order.json"}),
*     clients: Multipart::field({name:"clients", value: myClients, mime: "application/xml"})
*   }
* }
* ----
*
* ==== Output
*
* [source,txt,linenums]
* ----
* ------=_Part_8032_681891620.1542560124825
* Content-Type: application/json
* Content-Disposition: form-data; name="order"; filename="order.json"
*
* [
*   {
*     "order": 1,
*     "amount": 2
*   },
*   {
*     "order": 32,
*     "amount": 1
*   }
* ]
* ------=_Part_8032_681891620.1542560124825
* Content-Type: application/xml
* Content-Disposition: form-data; name="clients"
*
* <clients>
*   <client>
*     <id>1</id>
*     <name>Mariano</name>
*   </client>
*   <client>
*     <id>2</id>
*     <name>Shoki</name>
*   </client>
* </clients>
* ------=_Part_8032_681891620.1542560124825--
* ----
*/
fun field(opts: {|name: String, value: Any, mime?: String, fileName?: String |}): MultipartPart =  do {
    field(opts.name, opts.value, opts.mime default '', opts.fileName default '')
}

/**
* Creates a `MultipartPart` data structure using the specified part name,
* input content for the part, format (or mime type), and optionally, file name.
*
*
* This version of the `field` function accepts arguments in a comma-separated
* list, for example:
*
* [source,txt,linenums]
* ----
* Multipart::field("order", myOrder,"application/json", "order.json")`
* ----
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name | Description
* | opts | A set of parameters that specify:
*
* * A unique `name` (required) for the `Content-Disposition` header of the part.
* * A `value` (required) for the content of the part.
* * `mime` (optional for strings) for the mime type (for example, `application/json`) to apply to content within the part. This type can be used to transform the input type.
* * An optional `fileName` value that you can supply to the `filename` parameter in the part's `Content-Disposition` header.
* |===
*
* === Example
*
* This example produces two parts. The first part (named `order`) outputs
* content in JSON and provides a file name for the part (`order.json`). The
* second (named `clients`) outputs content in XML and does not provide a file
* name. The only difference between this `field` example and the previous
* `field` example is the way you pass in arguments to the method. Also notice
* that in this example you need to add the function's namespace to the function
* name, for example, `Multipart::field`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::module::Multipart
* output multipart/form-data
* var myOrder = [
*   {
*     order: 1,
*     amount: 2
*   },
*   {
*     order: 32,
*     amount: 1
*   }
* ]
* var myClients = {
*   clients: {
*     client: {
*       id: 1,
*       name: "Mariano"
*     },
*     client: {
*       id: 2,
*       name: "Shoki"
*     }
*   }
* }
* ---
* {
*   parts: {
*     order: Multipart::field("order", myOrder, "application/json", "order.json"),
*     clients: Multipart::field("clients", myClients, "application/xml")
*   }
* }
* ----
*
* ==== Output
*
* [source,txt,linenums]
* ----
* ------=_Part_4846_2022598837.1542560230901
* Content-Type: application/json
* Content-Disposition: form-data; name="order"; filename="order.json"
*
* [
*   {
*     "order": 1,
*     "amount": 2
*   },
*   {
*     "order": 32,
*     "amount": 1
*   }
* ]
* ------=_Part_4846_2022598837.1542560230901
* Content-Type: application/xml
* Content-Disposition: form-data; name="clients"
*
* <clients>
*   <client>
*     <id>1</id>
*     <name>Mariano</name>
*   </client>
*   <client>
*     <id>2</id>
*     <name>Shoki</name>
*   </client>
* </clients>
* ------=_Part_4846_2022598837.1542560230901--
* ----
*/
fun field(name: String, value: Any, mime: String = "", fileName: String = ""): MultipartPart =
  {
    headers: {
      ("Content-Type": mime) if mime != '',
      "Content-Disposition": {
        "name": name,
        ("filename": fileName) if fileName != ''
      }
    },
    content: value
  }

/**
* Creates a `MultipartPart` data structure from a resource file.
*
*
* This version of the `file` function accepts as argument an object containing key/value pairs, enabling you to enter the key/value pairs in any order, for example:
*
* [source,txt,linenums]
* ----
* Multipart::file({ name: "myFile", path: "myClients.json", mime: "application/json", fileName: "partMyClients.json"})
* ----
*
* === Parameters
*
* [%header, cols="1a,3a"]
* |===
* | Name | Description
* | opts | An object that specifies the following key/value pairs:
*
* * A unique `name` (required) for the `Content-Disposition` header of the part.
* * A `path` (required) relative to the `src/main/resources` project path for the Mule app.
* * `mime` (optional for strings) for the mime type (for example, `application/json`) to apply to content within the part. This setting _cannot_ be used to transform the input mime type.
* * An optional `fileName` value for the `filename` parameter in the part's `Content-Disposition` header. Defaults to the string `"filename"` if not
supplied. This value does not need to match the input file name.
* |===
*
* === Example
*
* This example creates a `MultipartPart` from a file accessible to the DataWeave function, the file name is `orders.xml` and is located in the Mule application's `/src/main/resources` folder.
*
* The `file` function locates the external `orders.xml` file and uses key/value pairs to indicate the various parameters needed to build the `MultipartPart`.
*
* * The `name` can be anything, but it usually coincides with the required parameter needed by the receiving server that accepts this `Multipart` payload.
* * The `path` is set to `./orders.xml`, which is the path and name for the `orders.xml` file that is loaded into the `MultipartPart`.
* * The `mime` parameter specifies the correct MIME type for the file. In this case, it is `application/xml`.
* * The `filename` can be changed to any value, it can be different from the actual input file's filename.
*
* Note that the output of this example is not compatible with the `multipart/form-data` output type because it is just one part of a `Multipart` structure. To create a valid `multipart/form-data` output, use the `Multipart::form()` function with one or more `Multipart` files and/or fields.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::module::Multipart
* output application/dw
* var ordersFilePath = "./orders.xml"
* ---
* Multipart::file{ name: "file", path: ordersFilePath, mime: "application/xml", fileName: "orders.xml" }
* ----
*
* ==== Input
*
* A file called `orders.xml` located in `src/main/resources` with the following content:
*
* [source,xml,linenums]
* ----
* <orders>
*   <order>
*     <item>
*       <id>1001</id>
*       <qty>1</qty>
*       <price>\$100</price>
*     </item>
*     <item>
*       <id>2001</id>
*       <qty>2</qty>
*       <price>\$50</price>
*     </item>
*   </order>
* </orders>
* ----
*
* ==== Output
*
* [source,json,linenums]
* ----
* {
* headers: {
*     "Content-Type": "application/xml",
*     "Content-Disposition": {
*       name: "file",
*       filename: "orders.xml"
*     }
*   },
*   content: "<?xml version='1.0' encoding='UTF-8'?>\n<orders>\n  <order>\n    <item>\n      <id>1001</id>\n      <qty>1</qty>\n      <price>\$100</price>\n    </item>\n    <item>\n      <id>2001</id>\n      <qty>2</qty>\n      <price>\$50</price>\n    </item>\n  </order>\n</orders>"
* }
* ----
*
* === Example
*
* This example inserts file content from a `MultipartPart` into a `Multipart`, resulting in a `multipart/form-data` output. The example uses the `form` function to create the `Multipart` and uses `file` to create a part.
*
* The `Multipart::form()` function accepts an array of `Multipart` items, where each part can be created using the `Multipart::field()` or `Multipart::file()` functions.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::module::Multipart
* output multipart/form-data
* var ordersFilePath = "./orders.xml"
* var myArgs = { name: "file", path: ordersFilePath, mime: "application/xml", fileName: "myorders.xml"}
* ---
* Multipart::form([
*   Multipart::file(myArgs)
* ])
* ----
* ==== Output
*
* [source,json,linenums]
* ----
* ------=_Part_5349_1228640551.1560391284935
* Content-Type: application/xml
* Content-Disposition: form-data; name="file"; filename="myorders.xml"
* <?xml version='1.0' encoding='UTF-8'?>
* <orders>
*   <order>
*     <item>
*       <id>1001</id>
*       <qty>1</qty>
*       <price>$100</price>
*     </item>
*     <item>
*       <id>2001</id>
*       <qty>2</qty>
*       <price>$50</price>
*     </item>
*   </order>
* </orders>
* ------=_Part_5349_1228640551.1560391284935--
* ----
*
*/
fun file(opts: {| name: String, path: String, mime?: String, fileName?: String |}) =
    file(opts.name, opts.path, opts.mime default 'application/octet-stream', opts.fileName default 'filename')

/**
* Creates a `MultipartPart` data structure from a resource file.
*
*
* This version of the `file` function accepts String arguments in a comma-separated
* list, for example:
*
* [source,txt,linenums]
* ----
* Multipart::field("myFile", myClients, 'application/json', "partMyClients.json")
* ----
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name | Description
* | `fieldName` | A unique name (required) for the `Content-Disposition` header of the part.
* | `path` | A path (required) relative to the `src/main/resources` project path for the Mule app.
* | `mime` | The mime type (optional for strings), for example, `application/json`, to apply to content within the part. This setting _cannot_ be used to transform the input mime type.
* | `sentFileName` | An optional file name value for the `filename` parameter in the part's `Content-Disposition` header. Defaults to the string `"filename"` if not specified. This value does not need to match the input file name.
* |===
*
* === Example
*
* This example inserts file content from a `MultipartPart` into a `Multipart`
* data structure. It uses the `form` function to create the `Multipart` type
* and uses `file` to create a part named `myClient` with JSON content from
* an external file `myClients.json`. It also specifies `partMyClients.json` as
* the value for to the `filename` parameter.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::module::Multipart
* var myClients = "myClients.json"
* output multipart/form-data
* ---
* Multipart::form([
*  Multipart::file("myFile", myClients, 'application/json', "partMyClients.json")
* ])
* ----
*
* ==== Input
*
* A file called `myClients.json` and located in `src/main/resources` with the
* following content.
*
* [source,JSON,linenums]
* ----
* clients: {
*     client: {
*       id: 1,
*       name: "Mariano"
*     },
*     client: {
*       id: 2,
*       name: "Shoki"
*     }
*   }
* ----
*
* ==== Output
*
* [source,txt,linenums]
* ----
* ------=_Part_1586_1887987980.1542569342438
* Content-Type: application/json
* Content-Disposition: form-data; name="myFile"; filename="partMyClients.json"
*
* {
*    clients: {
*      client: {
*        id: 1,
*        name: "Mariano"
*      },
*      client: {
*        id: 2,
*        name: "Shoki"
*      }
*    }
* }
* ------=_Part_1586_1887987980.1542569342438--
* ----
*/
fun file(fieldName: String, path: String, mime: String = 'application/octet-stream', sentFileName: String = 'filename') =
  field(fieldName, readUrl('classpath://$(path)', 'application/octet-stream') as Binary, mime, sentFileName)

/**
* Creates a `Multipart` data structure using a specified array of parts.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name | Description
* | parts | An array of parts (`MultipartPart` data structures).
* |===
*
* === Example
* This example creates a `Multipart` data structure that contains three parts.
*
* The first part uses the `Multipart::file()` function to import an external file named `orders.xml`. The file is located in the internal `src/main/resources` folder of the Mule application. See the xref:dw-multipart-functions-file.adoc[file] function documentation for more details on this example.
*
* The second part uses the `Multipart::field()` function version that accepts field names as input parameters in the form of an object with key/value pairs, enabling you to pass the keys in any order. This part also does not specify the optional `fileName` parameter. When specified, `fileName` is part of the `Content-Distribution` element of the part. The `mime` field is also optional. When included, the field sets the `Content-Type` element to the `mime` value. In this case the `Content-Type` is set to `text/plain`.
*
* The third part uses the more compact version of the `Multipart::field()` function which sets the required and optional parameters, in the correct order, as input parameters. The first three parameters `name`, `value`, and `mime` are required. The `fileName` parameters is optional, use it only if the content is read from a file or is written to a file. In this version, the `mime` parameter is output as the `Content-Type` element, and the `fileName` is output as the `filename` parameter of the `Content-Distribution` element.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::module::Multipart
* output multipart/form-data
* var myOrders = "./orders.xml"
* var fileArgs = { name: "file", path: myOrders, mime: "application/xml", fileName: "myorders.xml"}
* var fieldArgs = {name:"userName",value: "Annie Point", mime: "text/plain"}
* ---
* Multipart::form([
*   Multipart::file(fileArgs),
*   Multipart::field(fieldArgs),
*   Multipart::field("myJson", {"user": "Annie Point"}, "application/json", "userInfo.json")
* ])
* ----
*
* ==== Output
*
* [source,json,linenums]
* ----
* ------=_Part_146_143704079.1560394078604
* Content-Type: application/xml
* Content-Disposition: form-data; name="file"; filename="myorders.xml"
* <?xml version='1.0' encoding='UTF-8'?>
* <orders>
*   <order>
*     <item>
*       <id>1001</id>
*       <qty>1</qty>
*       <price>$100</price>
*     </item>
*     <item>
*       <id>2001</id>
*       <qty>2</qty>
*       <price>$50</price>
*     </item>
*   </order>
* </orders>
* ------=_Part_146_143704079.1560394078604
* Content-Type: text/plain
* Content-Disposition: form-data; name="userName"
* Annie Point
* ------=_Part_146_143704079.1560394078604
* Content-Type: application/json
* Content-Disposition: form-data; name="myJson"; filename="userInfo.json"
* {
*   "user": "Annie Point"
* }
* ------=_Part_146_143704079.1560394078604--
* ----
*
* === Example
*
* This example constructs a data structure using DataWeave code that is compatible with the `multipart/form-data` output format, demonstrating how you can manually construct a data structure compatible with `multipart/form-data` output, without using the `form` function.
*
* In the following structure, the part keys `part1` and `part2` are stripped out in the `multipart/form-data` output.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::module::Multipart
* output multipart/form-data
* var firstPart = "content for my first part"
* var secondPart = "content for my second part"
* ---
* {
*   parts: {
*     part1: Multipart::field({name:"myFirstPart",value: firstPart}),
*     part2: Multipart::field("mySecondPart", secondPart)
*   }
* }
* ----
*
* ==== Output
*
* [source,txt,linenums]
* ----
* ------=_Part_320_1528378161.1542639222352
* Content-Disposition: form-data; name="myFirstPart"
*
* content for my first part
* ------=_Part_320_1528378161.1542639222352
* Content-Disposition: form-data; name="mySecondPart"
*
* content for my second part
* ------=_Part_320_1528378161.1542639222352--
* ----
*/
fun form(parts: Array<MultipartPart>): Multipart =
  {
    parts: parts reduce ((val, carry = {}) ->
      carry ++
      { (val.headers['Content-Disposition'].name): val }
    )
  }


/**
* Helper function for generating boundaries in `Multipart` data structures.
*/
fun generateBoundary(len: Number = 70): String = do {
    /**
    * Default Boundary separator
    */
    var boundaryChars = "-_1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
    var maxChars = sizeOf(boundaryChars)
    ---
    (1 to len) map boundaryChars[floor(random() * maxChars)] joinBy ''
}
