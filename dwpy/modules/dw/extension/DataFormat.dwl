/**
* This module provides resources for registering a new data
* format for the DataWeave language.
*
* For an example, see
https://github.com/mulesoft-labs/data-weave-custom-data-format[Custom Data Formats Example].
*
*/
%dw 2.0

/**
* Registration hook that the DataWeave engine uses to discover the variable that represents the custom data format. For an example, see the https://github.com/mulesoft-labs/data-weave-custom-data-format/blob/master/README.md[Custom Data Formats Example README].
*/
@AnnotationTarget(targets = ["Variable"])
annotation DataFormatExtension()

/**
 * Represents a MIME type, such as `application/json`.
 */
@Since(version = "2.2.0")
type MimeType = String

/**
 * Reader or writer configuration settings.
 */
@Since(version = "2.2.0")
type Settings = {}

/**
 * Represents a configuration with no settings.
 */
@Since(version = "2.2.0")
type EmptySettings = {}

/**
 * Represents encoding settings and contains the following field:
 *
 * * `encoding`:
 *   Encoding that the writer uses for output. Defaults to "UTF-8".
 */
@Since(version = "2.2.0")
type EncodingSettings = {
    encoding?: String {defaultValue: "UTF-8"}
}

/**
 * Represents the `DataFormat` definition and contains the following fields:
 *
 * * `binaryFormat`:
 *    True if this is data format is represented as binary representation instead of text. False if not present.
 *
 * * `defaultCharset`:
 *    Default character set of this format, if any.
 *
 * * `fileExtensions`:
 *   Returns the list of file extensions with the `.` (for example, `.json`, `.xml`) to assign to this data format.
 *
 * * `acceptedMimeTypes`:
 *   The list of MIME types to accept.
 *
 * * `reader`:
 *   Function that reads raw content and transforms it into the canonical DataWeave model.
 *
 * * `writer`:
 *   Function that writes the canonical DataWeave model into binary content.
 */
@Since(version = "2.2.0")
type DataFormat<ReaderSettings <: Settings, WriterSettings <: Settings> = {
    binaryFormat?: Boolean,
    defaultCharset?: String,
    fileExtensions?: Array<String>,
    acceptedMimeTypes: Array<MimeType>,
    reader: (content: Binary, charset: String, settings: ReaderSettings) -> Any,
    writer: (value: Any, settings: WriterSettings) -> Binary
}
