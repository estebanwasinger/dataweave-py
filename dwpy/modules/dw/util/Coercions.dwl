/**
* This utility module assists with type coercions.
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::util::Coercions` to the header of your
* DataWeave script.
*/
@Since(version = "2.4.0")
%dw 2.0

import * from dw::Runtime

/**
* Type used when rounding decimal values up or down.
*/
type RoundingMode = "UP"| "DOWN" | "CEILING" | "FLOOR" | "HALF_UP" | "HALF_DOWN" | "HALF_EVEN"

/**
* Type used for setting units to `"milliseconds"` or `"seconds"`.
*/
type MillisOrSecs = "milliseconds" | "seconds"

/**
* Type used for setting units of a `Period` value  to `"hours"`, `"minutes"`, `"seconds"`,
* `"milliseconds"`, or `"nanos"`.
*/
type PeriodUnits = "hours" |  "minutes" | "seconds" | "milliseconds" | "nanos"

/**
* Type used for formatting `Dates` types and `Number`.
* Supports the following fields:
*
* * `format`: (optional) The ISO-8601 formatting to use on the date or time.
*             For example, this parameter accepts character patterns
*             based on the Java 8 `java.time.format`.
*             A `null` value has no effect on the value.
* * `locale`: (optional) ISO 3166 country code to use, such as `US`,
*             `AR`, or `ES`. A `null` or absent value uses your
*             JVM default. When you pass a translatable format, such as
*             `eeee` and `MMMM`, a `locale` (such as `ES`) transforms
*             the corresponding numeric values to a localized string.
*/
type Formatter = {
    format?: String,
    locale?: String
}

/**
* A variant of `toString` that transforms a `Number` value
* (whole or decimal) into a `String` value and accepts a
* format, locale, and rounding mode value.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | number | The `Number` value to format.
* | format | The formatting to apply to the `Number` value.
*            A `format` accepts `#` or `0` (but not both) as
*            placeholders for _decimal_ values, and only one decimal
*            point is permitted. A `null` or empty `String` value has
*            no effect on the `Number` value. Most other values are
*            treated as literals, but you must escape special
*            characters, such as a dollar sign (for example, `\$`).
*            Inner quotations must be closed and differ from the
*            surrounding quotations.
* | locale | Optional ISO 3166 country code to use, such as `US`,
*            `AR`, or `ES`. A `null` or absent value uses your
*            JVM default. When you pass a translatable format, such as
*            `eeee` and `MMMM`, a `locale` (such as `ES`) transforms
*            the corresponding numeric values to a localized string.
* | roundMode | Optional parameter for rounding decimal values
*               when the formatting presents a rounding choice,
*               such as a format of `0.#` for the decimal `0.15`.
*               The default is `HALF_UP`, and a `null` value
*               behaves like `HALF_UP`.
*               Only one of the following values is permitted:
*
* * `UP`: Always rounds away from zero (for example, `0.01` to `"0.1"`
*          and `-0.01` to `"-0.1"`). Increments the preceding digit
*          to a non-zero fraction and never decreases the magnitude of
*          the calculated value.
* * `DOWN`: Always rounds towards zero (for example, `0.19` to `"0.1"`
*           and `-0.19` to `"-0.1"`). Never increments the digit before
*           a discarded fraction (which truncates to the preceding
*           digit) and never increases the magnitude of the calculated
*           value.
* * `CEILING`: Rounds towards positive infinity and behaves like `UP`
*              if the result is positive (for example, `0.35` to `"0.4"`).
*              If the result is negative, this mode behaves like `DOWN`
*              (for example, `-0.35` to `"-0.3"`). This mode never
*              decreases the calculated value.
* * `FLOOR`: Rounds towards negative infinity and behaves like DOWN
*            if the result is positive (for example, `0.35` to `"0.3"`).
*            If the result is negative, this mode behaves like `UP`
*            (for example, `-0.35` to `"-0.4"`). The mode never increases
*            the calculated value.
* * `HALF_UP`: Default mode, which rounds towards the nearest
*              "neighbor" unless both neighbors are equidistant,
*              in which case, this mode rounds up. For example,
*              `0.35` rounds to `"0.4"`, `0.34` rounds to `"0.3"`, and
*              `0.36` rounds to `"0.4"`. Negative decimals numbers round
*              similarly. For example, `-0.35` rounds to `"-0.4`".
* * `HALF_DOWN`: Rounds towards the nearest numeric
*              "neighbor" unless both neighbors are equidistant,
*              in which case, this mode rounds down. For example,
*              `0.35` rounds to `"0.3"`, `0.34` rounds to `"0.3"`, and
*              `0.36` rounds to `"0.4"`. Negative decimals numbers round
*              similarly. For example, `-0.35` rounds to `"-0.3"`.
* * `HALF_EVEN`: For decimals that end in a `5` (such as, `1.125` and `1.135`),
*              the behavior depends on the number that precedes the `5`.
*              `HALF_EVEN` rounds up when the next-to-last digit before
*              the `5` is an odd number but rounds down when the next-to-last
*              digit is even. For example, `0.225` rounds to `"0.22"`, `0.235`
*              _and_ `0.245` round to `"0.24"`, and `0.255` rounds to `"0.26"`.
*              Negative decimals round similarly, for example, `-0.225` to
*              `"-0.22"`. When the last digit is not `5`, the setting behaves
*              like `HALF_UP`. Rounding of monetary values sometimes follows
*              the `HALF_EVEN` pattern.
* |===
*
* === Example
*
* This example shows how `toString` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/json
* ---
* {
*     a: toString(1.0),
*     b: toString(0.005,".00"),
*     c: toString(0.035,"#.##","ES"),
*     d: toString(0.005,"#.##","ES","HALF_EVEN"),
*     e: toString(0.035,"#.00",null,"HALF_EVEN"),
*     f: toString(1.1234,"\$.## 'in my account'")
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "a": "1",
*   "b": ".01",
*   "c": "0,04",
*   "d": "0",
*   "e": ".04",
*   "f": "$1.12 in my account"
* }
* ----
**/
@Labels(labels = ["format"])
@Since(version = "2.4.0")
fun toString(number: Number, format: String | Null = null, locale: String | Null = null, roundMode: RoundingMode | Null = null): String =
    number as String {
                        (format: format) if (format != null),
                        (locale: locale) if (locale != null),
                        (roundMode: roundMode) if (roundMode != null)
                    }

/**
* A variant of `toString` that transforms a `Date`, `DateTime`,
* `LocalTime`, `LocalDateTime`, or `Time` value into a `String` value.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | date   | The `Date`, `DateTime`, `LocalTime`, `LocalDateTime`,
*              or `Time` value to coerce to a `String` type.
* | format | The ISO-8601 formatting to use on the date or time.
*            For example, this parameter accepts character patterns
*            based on the Java 8 `java.time.format`.
*            A `null` value has no effect on the value. Defaults:
*
* * `Date` example: `2011-12-03` (equivalent format: `uuuu-MM-dd`)
* * `DateTime` example: `2011-12-03T10:15:30.000000+01:00` (equivalent format: `uuuu-MM-dd HH:mm:ssz`)
* * `LocalDateTime` example: `2011-12-03T10:15:30.000000` (equivalent format: `uuuu-MM-dd HH:mm:ss`)
* * `LocalTime` example: `10:15:30.000000` (equivalent format: `HH:mm:ss.n`)
* * `Time` example: `10:15:30.000000Z` (equivalent format: `HH:mm:ss.nxxxz`)
* | locale | Optional ISO 3166 country code to use, such as `US`,
*            `AR`, or `ES`. A `null` or absent value uses your
*            JVM default. When you pass a translatable format, such as
*            `eeee` and `MMMM`, a `locale` (such as `ES`) transforms
*            the corresponding numeric values to a localized string.
* |===
*
* === Example
*
* This example shows how `toString` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/json
* ---
* {
*    aDate: toString(|2003-10-01|, "uuuu/MM/dd"),
*    aDateTime: toString(|2018-09-17T22:13:00-03:00|),
*    aLocalTime: toString(|23:57:59|, "HH-mm-ss"),
*    aLocalDateTime : toString(|2015-10-01T23:57:59|),
*    aLocalDateTimeFormatted: toString(|2003-10-01T23:57:59|, "uuuu-MM-dd HH:mm:ss a"),
*    aLocalDateTimeFormattedAndLocalizedSpain: toString(|2003-01-01T23:57:59|, "eeee, dd MMMM, uuuu HH:mm:ss a", "ES"),
*    aTime: typeOf(|22:10:18Z|),
*    aTimeZone: toString(|-03:00|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "aDate": "2003/10/01",
*   "aDateTime": "2018-09-17T22:13:00-03:00",
*   "aLocalTime": "23-57-59",
*   "aLocalDateTime": "2015-10-01T23:57:59",
*   "aLocalDateTimeFormatted": "2003-10-01 23:57:59 PM",
*   "aLocalDateTimeFormattedAndLocalizedSpain": "miércoles, 01 enero, 2003 23:57:59 p. m.",
*   "aTime": "Time",
*   "aTimeZone": "-03:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun toString(date: Date | DateTime | LocalDateTime | LocalTime | Time, format: String | Null = null, locale: String | Null = null): String =
      date as String {
                        (format: format) if (format != null),
                        (locale: locale) if (locale != null)
                     }
/**
* A variant of `toString` that transforms a `Binary` value
* into a `String` value with the specified encoding.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | binary | The `Binary` value to coerce to a `String` value.
* | encoding | The encoding to apply to the `String` value. Accepts
*              encodings that are supported by your JDK. For example,
*              `encoding` accepts Java canonical names and aliases for
*              the basic and extended encoding sets in Oracle JDK 8 and
*              JDK 11.
* |===
*
* === Example
*
* This example shows how `toString` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* var binaryData= "DW Test" as Binary {encoding: "UTF-32"}
* output application/json
* ---
* {
*   a: toString(binaryData, "UTF-32"),
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "a": "DW Test"
* }
* ----
**/
@Since(version = "2.4.0")
fun toString(binary: Binary, encoding: String): String =
    binary as String {
                        encoding: encoding
                     }

/**
* A variant of `toString` that transforms a `TimeZone`, `Uri`,
* `Boolean`, `Period`, `Regex`, or `Key` value into a
* string.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | data | The `TimeZone`, `Uri`, `Boolean`, `Period`, `Regex`,
*          or `Key` value to coerce to a `String` value.
* |===
*
* === Example
*
* This example shows how `toString` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/json
* ---
* {
*   transformTimeZone: toString(|Z|),
*   transformBoolean: toString(true),
*   transformPeriod: toString(|P1D|),
*   transformRegex: toString(/a-Z/),
*   transformPeriod: toString(|PT8M10S|),
*   transformUri: toString("https://docs.mulesoft.com/" as Uri)
* }  ++
* { transformKey : toString((keysOf({ "aKeyToString" : "aValue"})[0])) }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "transformTimeZone": "Z",
*   "transformBoolean": "true",
*   "transformPeriod": "P1D",
*   "transformRegex": "a-Z",
*   "transformPeriod": "PT8M10S",
*   "transformUri": "https://docs.mulesoft.com/",
*   "transformKey": "aKeyToString"
* }
* ----
**/
@Since(version = "2.4.0")
fun toString(data: TimeZone | Uri | Boolean| Period| Regex| Key): String =
      data as String

/**
* A variant of `toString` that joins an `Array` of characters
* into a single `String` value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | arr | The `Array` of characters to transform into a `String` value.
* |===
*
* === Example
*
* This example shows how `toString` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/json
* ---
* {
*   a: toString([]),
*   b: toString(["h", "o", "l", "a"])
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "a": "",
*   "b": "hola"
* }
* ----
**/
@Since(version = "2.4.0")
fun toString(arr: Array<String>): String = arr joinBy ""

/**
* Splits a `String` value into an `Array` of characters.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | text | The `String` value to transform into an `Array`
*          of characters (a `Array<String>` type).
* |===
*
* === Example
*
* This example shows how `toArray` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/json indent=false
* ---
* {
*   a: toArray(""),
*   b: toArray("hola")
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {"a": [],"b": ["h","o","l","a"]}
* ----
**/
@Since(version = "2.4.0")
fun toArray(@StreamCapable text: String): Array<String> = text reduce ((item, acc=[]) -> acc ++ [item])


/***
*
* Transforms a `String` value into a `Number` value using the first `Formatter` that
* matches with the given value to transform.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `str` | String | The `String` value to transform into a `Number` value.
* | `formatters` | Array<DatesFormatter&#62; | The `array` of formatting to use on the `Number` value.
* |===
*
* === Example
*
* This example shows how `toNumber` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* import * from dw::Runtime
* output application/dw
* ---
* {
*   a: toNumber("0.005", [{format: "seconds"}, {format: ".00"}]),
*   b: try(() -> toNumber("0.005", [{format: "seconds"}])).error.message
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: 0.005 as Number {format: ".00"},
*   b: "Could not find a valid formatter for '0.005'"
* }
* ----
**/
@Since(version = "2.5.0")
fun toNumber(str: String, formatters: Array<Formatter>): Number = do {
  formatters match {
    case [] -> fail("Could not find a valid formatter for '$(str)'")
    case [head ~ tail] -> do {
        var maybeNumber: TryResult<Number> = try(() -> toNumber(str, head.format, head.locale))
        ---
        maybeNumber.result onNull toNumber(str, tail)
    }
  }
}

/***
*
* Transforms a `String` value into a `Number` value using the first `Formatter` that matches
* with the given value to transform.
*
* If none of the `Formatter` matches with the given value, the function returns a `null` value.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `str` | String | The `String` value to transform into a `Number` value.
* | `formatters` | Array<Formatter&#62; | The `array` of formatting to use on the `Number` value.
* |===
*
* === Example
*
* This example shows how `toNumberOrNull` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*   a: toNumberOrNull("0.005", [{format: "seconds"}, {format: ".00"}]),
*   b: toNumberOrNull("0.005", [{format: "seconds"}])
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: 0.005 as Number {format: ".00"},
*   b: null
* }
* ----
**/
@Since(version = "2.5.0")
fun toNumberOrNull(str: String, formatters: Array<Formatter>): Number | Null = try(() -> toNumber(str, formatters)).result

/**
* A variant of `toNumber` that transforms a `DateTime` value
* into a number of seconds or milliseconds, depending on the
* selected unit.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | dateTime | The `DateTime` value to transform into a `Number` value.
* | unit | The unit of time (`"milliseconds"` or `"seconds"`) to use
*          Given a `null` value, the function uses `"seconds"`.
* |===
*
* === Example
*
* This example shows how `toNumber` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/json
* ---
* {
*     epoch: toNumber(|2015-10-01T23:57:59Z|),
*     millis: toNumber(|2015-10-01T23:57:59Z|, "milliseconds")
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   "epoch": 1443743879,
*   "millis": 1443743879000
* }
* ----
**/
@Labels(labels = ["epoch", "currentTimeMillis"])
@Since(version = "2.4.0")
fun toNumber(dateTime: DateTime, unit: MillisOrSecs | Null = null): Number =
     dateTime as Number {
                           (unit: unit) if(unit != null)
                        }
/**
* A variant of `toNumber` that transforms a `Period` value
* into a number of hours, minutes, seconds, milliseconds
* or nanoseconds (`nanos`).
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | period | The `Period` value to transform into a `Number` value.
* | unit | The unit to apply to the specified `period`: `hours`,
*          `minutes`, `seconds`, `milliseconds`, or `nanos`.
* |===
*
* === Example
*
* This example shows how `toNumber` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/json
* ---
* {
*     toSecondsEx1: toNumber(|PT1H10M|, "seconds"),
*     toSecondsEx2: toNumber(|PT1M7S|, "milliseconds")
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "toSecondsEx1": 4200,
*   "toSecondsEx2": 67000
* }
* ----
**/
@Since(version = "2.4.0")
fun toNumber(period: Period, unit: PeriodUnits | Null = null): Number =
     period as Number {
                         (unit: unit) if(unit != null)
                      }


/**
* A variant of `toNumber` that transforms a `String` or `Key` value into
* a `Number` value and that accepts a format and locale.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | value | The `String` or `Key` value to transform into a `Number` value.
* | format | Optional formatting to apply to the `value`.
*            A `format` accepts `#` or `0` (but not both) as
*            placeholders for _decimal_ values and a single
*            whole number that is less than `10`. Only one decimal
*            point is permitted. A `null` or empty `String` value has
*            no effect on the `Number` value. Other characters
*            produce an error.
* | locale | Optional ISO 3166 country code to use, such as `US`,
*            `AR`, or `ES`. A `null` or absent value uses your
*            JVM default.
* |===
*
* === Example
*
* This example shows how `toNumber` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* var myKey = keysOf({"123" : "myValue"})
* output application/json
* ---
*  {
*      "default": toNumber("1.0"),
*      "withFormat": toNumber("0.005",".00"),
*      "withLocal": toNumber("1,25","#.##","ES"),
*      "withExtraPlaceholders": toNumber("5.55","####.####"),
*      "keyToNumber": toNumber(myKey[0])
*  }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "default": 1.0,
*   "withFormat": 0.005,
*   "withLocal": 1.25,
*   "withExtraPlaceholders": 5.55,
*   "keyToNumber": 123
* }
* ----
**/
@Since(version = "2.4.0")
fun toNumber(value: String | Key, format: String | Null = null, locale: String | Null = null  ): Number =
    value as Number {
                      (format: format) if (format != null),
                      (locale: locale) if (locale != null)
                   }

/***
*
* Transforms a `String` value into a `DateTime` value using the first `Formatter` that
* matches with the given value to transform.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `str` | String | The `String` value to transform into a `DateTime` value.
* | `formatters` | Array<Formatter&#62; | The `array` of formatting to use on the `DateTime` value.
* |===
*
* === Example
*
* This example shows how `toDateTime` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* import * from dw::Runtime
* output application/dw
* ---
* {
*   a: toDateTime("2003-10-01 23:57:59Z", [{format: "uuuu/MM/dd HH:mm:ssz"}, {format: "uuuu-MM-dd HH:mm:ssz"}]),
*   b: try(() -> toDateTime("2003-10-01 23:57:59Z", [{format: "uuuu/MM/dd HH:mm:ssz"}])).error.message
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |2003-10-01T23:57:59Z| as DateTime {format: "uuuu-MM-dd HH:mm:ssz"},
*   b: "Could not find a valid formatter for '2003-10-01 23:57:59Z'"
* }
* ----
**/
@Labels(labels = ["parseDateTime"])
@Since(version = "2.5.0")
fun toDateTime(str: String, formatters: Array<Formatter>): DateTime = do {
  formatters match {
    case [] -> fail("Could not find a valid formatter for '$(str)'")
    case [head ~ tail] -> do {
        var maybeDateTime: TryResult<DateTime> = try(() -> toDateTime(str, head.format, head.locale))
        ---
        maybeDateTime.result onNull toDateTime(str, tail)
    }
  }
}

/***
*
* Transforms a `String` value into a `DateTime` value using the first `Formatter` that matches
* with the given value to transform.
*
* If none of the `Formatter` matches with the given value, the function returns a `null` value.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `str` | String | The `String` value to transform into a `DateTime` value.
* | `formatters` | Array<Formatter&#62; | The `array` of formatting to use on the `DateTime` value.
* |===
*
* === Example
*
* This example shows how `toDateTimeOrNull` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*   a: toDateTimeOrNull("2003-10-01 23:57:59Z", [{format: "uuuu/MM/dd HH:mm:ssz"}, {format: "uuuu-MM-dd HH:mm:ssz"}]),
*   b: toDateTimeOrNull("2003-10-01 23:57:59Z", [{format: "uuuu/MM/dd HH:mm:ssz"}])
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |2003-10-01T23:57:59Z| as DateTime {format: "uuuu-MM-dd HH:mm:ssz"},
*   b: null
* }
* ----
**/
@Since(version = "2.5.0")
fun toDateTimeOrNull(str: String, formatters: Array<Formatter>): DateTime | Null = try(() -> toDateTime(str, formatters)).result

/**
* Transforms a `Number` value into a `DateTime` value
* using `milliseconds` or `seconds` as the unit.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | number | The `Number` value to transform into a `DateTime` value.
* | unit | The unit to use for the conversion: `"milliseconds"`
*          or `"seconds"`. A `null` value for the `unit` field
*          defaults to `"seconds"`.
* |===
*
* === Example
*
* This example shows how `toDateTime` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*     fromEpoch: toDateTime(1443743879),
*     fromMillis: toDateTime(1443743879000, "milliseconds")
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   fromEpoch: |2015-10-01T23:57:59Z|,
*   fromMillis: |2015-10-01T23:57:59Z| as DateTime {unit: "milliseconds"}
* }
* ----
**/
@Since(version = "2.4.0")
fun toDateTime(number: Number, unit: MillisOrSecs | Null = null): DateTime =
    number as DateTime {
                          (unit: unit) if(unit != null)
                       }

/**
* Transforms a `String` value into a `DateTime` value
* and accepts a format and locale.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | str | The `String` value to transform into a `DateTime` value.
* | format | The formatting to use on the `DateTime` value.
*            A `null` value has no effect on the `DateTime` value.
*            This parameter accepts Java character patterns based
*            on ISO-8601. A `DateTime` value, such as
*            `2011-12-03T10:15:30.000000+01:00`, has
*            the format `uuuu-MM-dd HH:mm:ssz`.
* | locale | Optional ISO 3166 country code to use, such as `US`,
*            `AR`, or `ES`. A `null` or absent value uses your
*            JVM default.
* |===
*
* === Example
*
* This example shows how `toDateTime` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*    a: toDateTime("2015-10-01T23:57:59Z"),
*    b: toDateTime("2003-10-01 23:57:59Z","uuuu-MM-dd HH:mm:ssz")
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |2015-10-01T23:57:59Z|,
*   b: |2003-10-01T23:57:59Z| as DateTime {format: "uuuu-MM-dd HH:mm:ssz"}
* }
* ----
**/
@Labels(labels = ["parseDateTime"])
@Since(version = "2.4.0")
fun toDateTime(str: String, format: String | Null = null, locale: String | Null = null ): DateTime =
    str as DateTime {
                       (format: format) if (format != null),
                       (locale: locale) if (locale != null)
                    }


/***
*
* Transforms a `String` value into a `LocalDateTime` value using the first `Formatter` that
* matches with the given value to transform.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `str` | String | The `String` value to transform into a `LocalDateTime` value.
* | `formatters` | Array<Formatter&#62; | The `array` of formatting to use on the `LocalDateTime` value.
* |===
*
* === Example
*
* This example shows how `toLocalDateTime` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* import * from dw::Runtime
* output application/dw
* ---
* {
*   a: toLocalDateTime("2003-10-01 23:57:59", [{format: "uuuu/MM/dd HH:mm:ss"}, {format: "uuuu-MM-dd HH:mm:ss"}]),
*   b: try(() -> toLocalDateTime("2003-10-01 23:57:59", [{format: "uuuu/MM/dd HH:mm:ss"}])).error.message
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |2003-10-01T23:57:59| as LocalDateTime {format: "uuuu-MM-dd HH:mm:ss"},
*   b: "Could not find a valid formatter for '2003-10-01 23:57:59'"
* }
* ----
**/
@Labels(labels = ["parseLocalDateTime"])
@Since(version = "2.5.0")
fun toLocalDateTime(str: String, formatters: Array<Formatter>): LocalDateTime = do {
  formatters match {
    case [] -> fail("Could not find a valid formatter for '$(str)'")
    case [head ~ tail] -> do {
        var maybeLocalDate: TryResult<LocalDateTime> = try(() -> toLocalDateTime(str, head.format, head.locale))
        ---
        maybeLocalDate.result onNull toLocalDateTime(str, tail)
    }
  }
}

/***
*
* Transforms a `String` value into a `LocalDateTime` value using the first `Formatter` that matches
* with the given value to transform.
*
* If none of the `Formatter` matches with the given value, the function returns a `null` value.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `str` | String | The `String` value to transform into a `LocalDateTime` value.
* | `formatters` | Array<Formatter&#62; | The `array` of formatting to use on the `LocalDateTime` value.
* |===
*
* === Example
*
* This example shows how `toLocalDateTimeOrNull` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*   a: toLocalDateTimeOrNull("2003-10-01 23:57:59", [{format: "uuuu/MM/dd HH:mm:ss"}, {format: "uuuu-MM-dd HH:mm:ss"}]),
*   b: toLocalDateTimeOrNull("2003-10-01 23:57:59", [{format: "uuuu/MM/dd HH:mm:ss"}])
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |2003-10-01T23:57:59| as LocalDateTime {format: "uuuu-MM-dd HH:mm:ss"},
*   b: null
* }
* ----
**/
@Since(version = "2.5.0")
fun toLocalDateTimeOrNull(str: String, formatters: Array<Formatter>): LocalDateTime | Null = try(() -> toLocalDateTime(str, formatters)).result

/**
* Transforms a `String` value into a `LocalDateTime` value
* and accepts a format and locale.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | str | The `String` value to transform into a `LocalDateTime` value.
* | format | The formatting to use on the `LocalDateTime` value.
*            A `null` value has no effect on the `LocalDateTime` value.
*            This parameter accepts Java character patterns based
*            on ISO-8601. A `LocalDateTime` value, such as
*            `2011-12-03T10:15:30.000000` has
*            the format `uuuu-MM-dd HH:mm:ss`.
* | locale | Optional ISO 3166 country code to use, such as `US`,
*            `AR`, or `ES`. A `null` or absent value uses your
*            JVM default.
* |===
*
* === Example
*
* This example shows how `toLocalDateTime` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*   a: toLocalDateTime("2015-10-01T23:57:59"),
*   b: toLocalDateTime("2003-10-01 23:57:59","uuuu-MM-dd HH:mm:ss")
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |2015-10-01T23:57:59|,
*   b: |2003-10-01T23:57:59| as LocalDateTime {format: "uuuu-MM-dd HH:mm:ss"}
* }
* ----
**/
@Labels(labels = ["parseLocalDateTime"])
@Since(version = "2.4.0")
fun toLocalDateTime(str: String, format: String | Null = null, locale: String | Null = null ): LocalDateTime =
    str as LocalDateTime {
                            (format: format) if (format != null),
                            (locale: locale) if (locale != null)
                         }

/***
*
* Transforms a `String` value into a `Date` value using the first `Formatter` that
* matches with the given value to transform.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `str` | String | The `String` value to transform into a `Date` value.
* | `formatters` | Array<Formatter&#62; | The `array` of formatting to use on the `Date` value.
* |===
*
* === Example
*
* This example shows how `toDate` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* import * from dw::Runtime
* output application/dw
* ---
* {
*   a: toDate("2023-28-03", [{format: "yyyy/MM/dd"}, {format: "yyyy-dd-MM", locale: "en_US"}]),
*   b: try(() -> toDate("2023-28-03", [{format: "yyyy/MM/dd"}])).error.message
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |2023-03-28| as Date {format: "yyyy-dd-MM", locale: "en_US"},
*   b: "Could not find a valid formatter for '2023-28-03'"
* }
* ----
**/
@Since(version = "2.5.0")
fun toDate(str: String, formatters: Array<Formatter>): Date = do {
  formatters match {
    case [] -> fail("Could not find a valid formatter for '$(str)'")
    case [head ~ tail] -> do {
        var maybeDate: TryResult<Date> = try(() -> toDate(str, head.format, head.locale))
        ---
        maybeDate.result onNull toDate(str, tail)
    }
  }
}

/***
*
* Transforms a `String` value into a `Date` value using the first `Formatter` that matches
* with the given value to transform.
*
* If none of the `Formatter` matches with the given value, the function returns a `null` value.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `str` | String | The `String` value to transform into a `Date` value.
* | `formatters` | Array<Formatter&#62; | The `array` of formatting to use on the `Date` value.
* |===
*
* === Example
*
* This example shows how `toDateOrNull` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*   a: toDateOrNull("2023-28-03", [{format: "yyyy/MM/dd"}, {format: "yyyy-dd-MM", locale: "en_US"}]),
*   b: toDateOrNull("2023-28-03", [{format: "yyyy/MM/dd"}])
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |2023-03-28| as Date {format: "yyyy-dd-MM", locale: "en_US"},
*   b: null
* }
* ----
**/
@Since(version = "2.5.0")
fun toDateOrNull(str: String, formatters: Array<Formatter>): Date | Null = try(() -> toDate(str, formatters)).result

/**
* Transforms a `String` value into a `Date` value
* and accepts a format and locale.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | str | The `String` value to transform into a `Date` value.
* | format | The formatting to use on the `Date` value.
*            A `null` value has no effect on the `Date` value.
*            This parameter accepts Java character patterns based
*            on ISO-8601. A `Date` value, such as
*           `2011-12-03`, has the format `uuuu-MM-dd`.
* | locale | Optional ISO 3166 country code to use, such as `US`,
*            `AR`, or `ES`. A `null` or absent value uses your
*            JVM default.
* |===
*
* === Example
*
* This example shows how `toDate` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*   a: toDate("2015-10-01"),
*   b: toDate("2003/10/01","uuuu/MM/dd")
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |2015-10-01|,
*   b: |2003-10-01| as Date {format: "uuuu/MM/dd"}
* }
* ----
**/
@Since(version = "2.4.0")
fun toDate(str: String, format: String | Null = null, locale: String | Null = null ): Date =
    str as Date {
                   (format: format) if (format != null),
                   (locale: locale) if (locale != null)
                }


/***
*
* Transforms a `String` value into a `Time` value using the first `Formatter` that
* matches with the given value to transform.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `str` | String | The `String` value to transform into a `Time` value.
* | `formatters` | Array<Formatter&#62; | The `array` of formatting to use on the `Time` value.
* |===
*
* === Example
*
* This example shows how `toTime` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* import * from dw::Runtime
* output application/dw
* ---
* {
*   a: toTime("13:44:12.283-08:00", [{format: "HH:mm:ss.xxx"}, {format: "HH:mm:ss.nxxx"}]),
*   b: try(() -> toTime("13:44:12.283-08:00", [{format: "HH:mm:ss.xxx"}]).error.message
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |13:44:12.000000283-08:00| as Time {format: "HH:mm:ss.nxxx"},
*   b: "Could not find a valid formatter for '13:44:12.283-08:00'"
* }
* ----
**/
@Since(version = "2.5.0")
fun toTime(str: String, formatters: Array<Formatter>): Time = do {
  formatters match {
    case [] -> fail("Could not find a valid formatter for '$(str)'")
    case [head ~ tail] -> do {
        var maybeTime: TryResult<Time> = try(() -> toTime(str, head.format, head.locale))
        ---
        maybeTime.result onNull toTime(str, tail)
    }
  }
}

/***
*
* Transforms a `String` value into a `Time` value using the first `Formatter` that matches
* with the given value to transform.
*
* If none of the `Formatter` matches with the given value, the function returns a `null` value.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `str` | String | The `String` value to transform into a `Time` value.
* | `formatters` | Array<Formatter&#62; | The `array` of formatting to use on the `Time` value.
* |===
*
* === Example
*
* This example shows how `toTimeOrNull` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*   a: toTimeOrNull("13:44:12.283-08:00", [{format: "HH:mm:ss.xxx"}, {format: "HH:mm:ss.nxxx"}]),
*   b: toTimeOrNull("13:44:12.283-08:00", [{format: "HH:mm:ss.xxx"}])
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |13:44:12.000000283-08:00| as Time {format: "HH:mm:ss.nxxx"},
*   b: null
* }
* ----
**/
@Since(version = "2.5.0")
fun toTimeOrNull(str: String, formatters: Array<Formatter>): Time | Null = try(() -> toTime(str, formatters)).result

/**
* Transforms a `String` value into a `Time` value
* and accepts a format and locale.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | str | The `String` value to transform into a `Time` value.
* | format | The formatting to use on the `Time` value.
*            A `null` value has no effect on the `Time` value.
*            This parameter accepts Java character patterns based
*            on ISO-8601. A `Time` value, such as
*            `10:15:30.000000`, has the format `HH:mm:ss.nxxx`.
* | locale | Optional ISO 3166 country code to use, such as `US`,
*            `AR`, or `ES`. A `null` or absent value uses your
*            JVM default.
* |===
*
* === Example
*
* This example shows how `toTime` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*    a: toTime("23:57:59Z"),
*    b: toTime("13:44:12.283-08:00","HH:mm:ss.nxxx")
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |23:57:59Z|,
*   b: |13:44:12.000000283-08:00| as Time {format: "HH:mm:ss.nxxx"}
* }
* ----
**/
@Since(version = "2.4.0")
fun toTime(str: String, format: String | Null = null, locale: String | Null = null ): Time =
    str as Time {
                   (format: format) if (format != null),
                   (locale: locale) if (locale != null)
                }

/***
*
* Transforms a `String` value into a `LocalTime` value using the first `Formatter` that
* matches with the given value to transform.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `str` | String | The `String` value to transform into a `LocalTime` value.
* | `formatters` | Array<Formatter&#62; | The `array` of formatting to use on the `LocalTime` value.
* |===
*
* === Example
*
* This example shows how `toLocalTime` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* import * from dw::Runtime
* output application/dw
* ---
* {
*   a: toLocalTime("23:57:59", [{format: "HH:mm:ss.n"}, {format: "HH:mm:ss"}]),
*   b: try(() -> toLocalTime("23:57:59", [{format: "HH:mm:ss.n"}])).error.message
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |23:57:59| as LocalTime {format: "HH:mm:ss"},
*   b: "Could not find a valid formatter for '23:57:59'"
* }
* ----
**/
@Since(version = "2.5.0")
fun toLocalTime(str: String, formatters: Array<Formatter>): LocalTime = do {
  formatters match {
    case [] -> fail("Could not find a valid formatter for '$(str)'")
    case [head ~ tail] -> do {
        var maybeLocalTime: TryResult<LocalTime> = try(() -> toLocalTime(str, head.format, head.locale))
        ---
        maybeLocalTime.result onNull toLocalTime(str, tail)
    }
  }
}

/***
*
* Transforms a `String` value into a `LocalTime` value using the first `Formatter` that matches
* with the given value to transform.
*
* If none of the `Formatter` matches with the given value, the function returns a `null` value.
*
* === Parameters
*
* [%header, cols="1,1,3"]
* |===
* | Name | Type | Description
* | `str` | String | The `String` value to transform into a `LocalTime` value.
* | `formatters` | Array<Formatter&#62; | The `array` of formatting to use on the `LocalTime` value.
* |===
*
* === Example
*
* This example shows how `toLocalTimeOrNull` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*   a: toLocalTimeOrNull("23:57:59", [{format: "HH:mm:ss.n"}, {format: "HH:mm:ss"}]),
*   b: toLocalTimeOrNull("23:57:59", [{format: "HH:mm:ss.n"}])
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   a: |23:57:59| as LocalTime {format: "HH:mm:ss"},
*   b: null
* }
* ----
**/
@Since(version = "2.5.0")
fun toLocalTimeOrNull(str: String, formatters: Array<Formatter>): LocalTime | Null = try(() -> toLocalTime(str, formatters)).result

/**
* Transforms a `String` value into a `LocalTime` value
* and accepts a format and locale.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | str | The `String` value to transform into a `LocalTime` value.
* | format | The formatting to use on the `LocalTime` value.
*            A `null` value has no effect on the `LocalTime` value.
*            This parameter accepts Java character patterns based
*            on ISO-8601. A `LocalTime` value, such as
*            `22:15:30.000000`, has the format `HH:mm:ss.n`.
* | locale | Optional ISO 3166 country code to use, such as `US`,
*            `AR`, or `ES`. A `null` or absent value uses your
*            JVM default.
* |===
*
* === Example
*
* This example shows how `toLocalTime` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/json
* ---
* {
*    toLocalTimeEx: toLocalTime("23:57:59"),
*    toLocalTimeEx2: toLocalTime("13:44:12.283","HH:mm:ss.n")
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   "toLocalTimeEx": "23:57:59",
*   "toLocalTimeEx2": "13:44:12.283"
* }
* ----
**/
@Since(version = "2.4.0")
fun toLocalTime(str: String, format: String | Null = null, locale: String | Null = null ): LocalTime =
    str as LocalTime {
                        (format: format) if (format != null),
                        (locale: locale) if (locale != null)
                     }

/**
*  Transform a `String` value into a `Period` value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | str | The `String` value to transform into a `Period` value.
* |===
*
* === Example
*
* This example shows how `toPeriod` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*   toPeriodEx1: toPeriod("P1D"),
*   toPeriodEx2: toPeriod("PT1H1M")
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   toPeriodEx1: |P1D|,
*   toPeriodEx2: |PT1H1M|
* }
* ----
**/
@Since(version = "2.4.0")
fun toPeriod(str: String): Period =
    str as Period

/**
*  Transforms a `String` value into a `Regex` value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | str | The `String` value to transform into a `Regex` value.
* |===
*
* === Example
*
* This example shows how `toRegex` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*   toRegexEx1: toRegex("a-Z"),
*   toRegexEx2: toRegex("0-9+")
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   toRegexEx1: /a-Z/,
*   toRegexEx2: /0-9+/
* }
* ----
**/
@Since(version = "2.4.0")
fun toRegex(str: String): Regex =
    str as Regex


/**
* Transform a `String` value into a `Boolean` value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | str | The `String` value to transform into a `Boolean` value.
* |===
*
* === Example
*
* This example shows how `toBoolean` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/json
* ---
* {
*   a: toBoolean("true"),
*   b: toBoolean("false"),
*   c: toBoolean("FALSE"),
*   d: toBoolean("TrUe")
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
*   "c": false,
*   "d": true
* }
* ----
**/
@Since(version = "2.4.0")
fun toBoolean(str: String): Boolean =
    str as Boolean


/**
* Transform a `String` value into a `TimeZone` value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | str | The `String` value to transform into a `TimeZone` value.
* |===
*
* === Example
*
* This example shows how `toTimeZone` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*    toTimeZoneOffset: toTimeZone("-03:00"),
*    toTimeZoneAbbreviation: toTimeZone("Z"),
*    toTimeZoneName: toTimeZone("America/Argentina/Buenos_Aires")
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   toTimeZoneOffset: |-03:00|,
*   toTimeZoneAbbreviation: |Z|,
*   toTimeZoneName: |America/Argentina/Buenos_Aires|
* }
* ----
**/
@Since(version = "2.4.0")
fun toTimeZone(str: String): TimeZone =
    str as TimeZone

/**
* Transforms a `String` value into a `Uri` value.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | str | The `String` value to transform into a `Uri` value.
* |===
*
* === Example
*
* This example shows how `toUri` behaves.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/json
* ---
* {
*   toUriExample: toUri("https://www.google.com/")
* }
* ----
*
* ==== Output
*
* [source,DataWeave,linenums]
* ----
* {
*   "toUriExample": "https://www.google.com/"
* }
* ----
**/
@Since(version = "2.4.0")
fun toUri(str: String): Uri =
  str as Uri

/**
* Transform a `String` value into a `Binary` value
* using the specified encoding.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name   | Description
* | str | The `String` value to transform into a `Binary` value.
* | encoding | The encoding to apply to the `String` value. Accepts
*              encodings that are supported by your JDK. For example,
*              `encoding` accepts Java canonical names and aliases for
*              the basic and extended encoding sets in Oracle JDK 8 and
*              JDK 11.
* |===
*
* === Example
*
* This example shows how `toBinary` behaves with different inputs.
* It produces output in the `application/dw` format.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Coercions
* output application/dw
* ---
* {
*   'UTF-16Ex': toBinary("DW", "UTF-16"),
*   'utf16Ex': toBinary("DW", "utf16"),
*   'UnicodeBigEx': toBinary("DW", "UnicodeBig"),
*   'UTF-32Ex': toBinary("DW", "UTF-32"),
*   'UTF_32Ex': toBinary("DW", "UTF_32")
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "UTF-16Ex": "/v8ARABX" as Binary {base: "64"},
*   utf16Ex: "/v8ARABX" as Binary {base: "64"},
*   UnicodeBigEx: "/v8ARABX" as Binary {base: "64"},
*   "UTF-32Ex": "AAAARAAAAFc=" as Binary {base: "64"},
*   UTF_32Ex: "AAAARAAAAFc=" as Binary {base: "64"}
* }
* ----
**/
@Since(version = "2.4.0")
fun toBinary(str: String, encoding: String): Binary =
  str as Binary {
                   encoding: encoding
                }
