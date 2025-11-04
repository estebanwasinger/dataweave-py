/**
* This module contains helper functions for working with XML doctype declarations.
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::xml::Dtd` to the header of your
* DataWeave script.
*/
@Since(version = "2.5.0")
%dw 2.0

/**
 * DataWeave type for representing a doctype declaration that is part of an XML file.
 * Supports the following fields:
 *
 * * `rootName`: Root element of the declaration.
 * * `publicId`: Publicly available standard (optional).
 * * `systemId`: Local URL (optional).
 * * `internalSubset`: Internal DTD subset (optional).
 */
@Since(version = "2.5.0")
type DocType = {
   rootName: String,
   publicId?: String,
   systemId?: String,
   internalSubset?: String,
}

/**
* Transforms a `DocType` value to a string representation.
*
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | docType | DocType | The `DocType` value to transform to a string.
* |===
*
* === Example
*
* This example transforms a `DocType` value that includes a `systemId` to a string representation.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::xml::Dtd
* output application/json
* ---
* docTypeAsString({rootName: "cXML", systemId: "http://xml.cxml.org/schemas/cXML/1.2.014/cXML.dtd"})
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* "cXML SYSTEM http://xml.cxml.org/schemas/cXML/1.2.014/cXML.dtd"
* ----
*
* === Example
*
* This example transforms a `DocType` value that includes a `publicId` and `systemId` to a string representation.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::xml::Dtd
* output application/json
* ---
* docTypeAsString({rootName: "html", publicId: "-//W3C//DTD XHTML 1.0 Transitional//EN", systemId: "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"})
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* "html PUBLIC -//W3C//DTD XHTML 1.0 Transitional//EN http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"
* ----
**/
@Since(version = "2.5.0")
fun docTypeAsString(docType: DocType): String = do {
    var identifier: String = if (docType.systemId?) do {
        var prefix: String =
        if (docType.publicId?) do {
            " PUBLIC " ++ docType.publicId! ++ " "
        } else do {
            " SYSTEM "
        }
        ---
        prefix ++ docType.systemId!

    } else ""
    var internalSubset: String = if (docType.internalSubset?) " [" ++ docType.internalSubset! ++ "]" else ""
    ---
    "$(docType.rootName)$(identifier)$(internalSubset)"
}
