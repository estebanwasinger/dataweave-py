/**
* This module contains functions for creating and manipulating dates.
*
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::core::Dates` to the header of your
* DataWeave script.
*/
@Since(version = "2.4.0")
%dw 2.0

import * from dw::core::Periods
import failIf from dw::Runtime

/**
* Returns the date for today as a `Date` type.
*
* === Example
*
* This example shows the output of `today` function.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* today()
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* "2021-05-15"
* ----
**/
@Since(version = "2.4.0")
fun today(): Date = now() as Date

/**
* Returns the date for yesterday as a `Date` type.
*
* === Example
*
* This example shows the output of `yesterday` function.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* yesterday()
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* "2021-05-14"
* ----
**/
@Since(version = "2.4.0")
fun yesterday(): Date = today() - days(1)

/**
* Returns the date for tomorrow as a `Date` type.
*
* === Example
*
* This example shows the output of `tomorrow` function.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import tomorrow from dw::core::Dates
* output application/json
* ---
* tomorrow()
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* "2021-05-16"
* ----
**/
@Since(version = "2.4.0")
fun tomorrow(): Date = today() + days(1)

/**
 * Type containing a selectable `timeZone` key and `TimeZone` value, such as
 * `{ timezone : &#124;-03:00&#124;}`.
 */
type Zoned = {timeZone: TimeZone}

/**
  * Type containing selectable `day`, `month`, and `year` keys and
  * corresponding `Number` values, such as `{day: 21, month: 1, year: 2021}`.
  * The fields accept a `Number` value.  Numbers preceded by `0`, such as `01`,
  * are not valid.
  */
type DateFactory =  {day: Number, month: Number, year: Number}

/**
 * Type containing selectable `hour`, `minutes`, and `seconds` keys and
 * corresponding `Number` values, such as `{hour: 8, minutes: 31, seconds: 55}`.
 * The fields accept any `Number` value.
 */
type LocalTimeFactory =  {hour: Number, minutes: Number, seconds: Number}

/**
 * Type that combines `LocalTimeFactory` and `Zoned` types. For example,
 * `{hour: 8, minutes: 31, seconds: 55, timeZone : &#124;-03:00&#124;} as TimeFactory`
 * is a valid `TimeFactory` value.
 */
type TimeFactory = LocalTimeFactory & Zoned

/**
 * Type that combines `DateFactory`, `LocalTimeFactory`, and `Zoned` types. For example,
 * `{day: 21, month: 1, year: 2021, hour: 8, minutes: 31, seconds: 55, timeZone : &#124;-03:00&#124;} as DateTimeFactory`
 * is a valid `DateTimeFactory` value.
 */
type DateTimeFactory = DateFactory & LocalTimeFactory & Zoned

/**
 * Type that combines `DateFactory` and `LocalTimeFactory` types. For example,
 * `{day: 21, month: 1, year: 2021, hour: 8, minutes: 31, seconds: 55, timeZone : &#124;-03:00&#124;} as LocalDateTimeFactory`
 * is a valid `LocalDateTimeFactory` value. The `timeZone` field is optional.
 */
type LocalDateTimeFactory = DateFactory & LocalTimeFactory

/**
* Creates a `Date` value from values specified for `year`, `month`, and `day` fields.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | parts | `Number` values for `year`, `month`, and `day` fields. The `month`
*           must be a value between 1 and 12, and the `day` value must be
*           between 1 and 31. You can specify the name-value pairs in any order,
*           but the output is ordered by default as a Date value, such as
*           `2012-10-11`. The input fields are parts of a `DateFactory` type.
* |===
*
* === Example
*
* This example shows how to create a value of type `Date`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*    newDate: date({year: 2012, month: 10, day: 11})
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "newDate": "2012-10-11"
* }
* ----
**/
@Since(version = "2.4.0")
fun date(parts: DateFactory): Date = do {
    var nYear = parts.year as String
    var nMonth = failIf( parts.month , (month) -> month <= 0 or month > 12, "Field 'month': `$(parts.month)` must be between 1 and 12.") as String {format: "00"}
    var nDay = failIf( parts.day, (day) -> day <= 0 or day > 31, "Field 'day': `$(parts.day)` must be between 1 and 31.") as String {format: "00"}
    ---
    "$(nYear)-$(nMonth)-$(nDay)" as Date
}


/**
* Creates a `DateTime` value from values specified for `year`, `month`, `day`, `hour`,
* `minutes`, `seconds`, and `timezone` fields.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | parts | `Number` values for `year`, `month`, `day`, `hour`, `minutes`, and
*           `seconds` fields followed by a `TimeZone` value for the the `timezone`
*           field. Valid values are numbers between 1 and 12 for the `month`,
*           1 through 31 for the `day`, 0 through 23 for the `hour`,
*           0 through 59 for `minutes`, and 0 through 59 (including decimals, such as 59.99) for seconds. You can specify the name-value pairs in any
*           order, but the output is ordered by default as a `DateTime` value,
*           such as `2012-10-11T10:10:10-03:00`. The input fields are parts of
*           a `DateTimeFactory` type.
* |===
*
* === Example
*
* This example shows how to create a value of type `DateTime`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*     newDateTime: dateTime({year: 2012, month: 10, day: 11, hour: 12, minutes: 30, seconds: 40 , timeZone: |-03:00|})
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "newDateTime": "2012-10-11T12:30:40-03:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun dateTime(parts: DateTimeFactory): DateTime = do {
     var nYear = parts.year as String
     var nMonth = failIf( parts.month , (month) -> month <= 0 or month > 12, "Field 'month': `$(parts.month)` must be between 1 and 12.") as String {format: "00"}
     var nDay = failIf( parts.day, (day) -> day <= 0 or day > 31, "Field 'day': `$(parts.day)` must be between 1 and 31.") as String {format: "00"}
     var nHours = failIf( parts.hour, (hour) -> hour < 0 or hour > 23, "Field 'hours': `$(parts.hour)` must be between 0 and 23.") as String {format: "00"}
     var nMinutes = failIf( parts.minutes, (minutes) -> minutes < 0 or minutes > 59, "Field 'minutes': `$(parts.minutes)` must be between 0 and 59.") as String {format: "00"}
     var nSeconds = failIf( parts.seconds, (seconds) -> seconds < 0 or seconds > 59, "Field 'seconds': `$(parts.seconds)` must be between 0 and 59.") as String {format: "00"}
     ---
     "$(nYear)-$(nMonth)-$(nDay)T$(nHours):$(nMinutes):$(nSeconds)$(parts.timeZone as String)" as DateTime
}


/**
* Creates a `LocalDateTime` value from values specified for `year`, `month`, `day`,
* `hour`, `minutes`, and `seconds` fields.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | parts | `Number` values for `year`, `month`, `day`, `hour`, `minutes`, and
*           `seconds` fields. Valid values are numbers between 1 and 12 for the
*           `month`, 1 through 31 for the `day`, 0 through 23 for the `hour`,
*           0 through 59 for `minutes`, and 0 through 59 (including decimals, such as 59.99) for `seconds` fields.
*           You can specify the name-value pairs in any order,
*           but the output is ordered as a default `LocalDateTime` value,
*           such as `2012-10-11T10:10:10`. The input fields are parts of
*           a `LocalDateTimeFactory` type.
* |===
*
* === Example
*
* This example shows how to create a value of type `LocalDateTime`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*     newLocalDateTime: localDateTime({year: 2012, month: 10, day: 11, hour: 12, minutes: 30, seconds: 40})
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "newLocalDateTime": "2012-10-11T12:30:40"
* }
* ----
**/
@Since(version = "2.4.0")
fun localDateTime(parts: LocalDateTimeFactory): LocalDateTime = do {
    var nYear = parts.year as String
    var nMonth = failIf( parts.month , (month) -> month <= 0 or month > 12, "Field 'month': `$(parts.month)` must be between 1 and 12.") as String {format: "00"}
    var nDay = failIf( parts.day, (day) -> day <= 0 or day > 31, "Field 'day': `$(parts.day)` must be between 1 and 31.") as String {format: "00"}
    var nHours = failIf( parts.hour, (hour) -> hour < 0 or hour > 23, "Field 'hours': `$(parts.hour)` must be between 0 and 23.") as String {format: "00"}
    var nMinutes = failIf( parts.minutes, (minutes) -> minutes < 0 or minutes > 59, "Field 'minutes': `$(parts.minutes)` must be between 0 and 59.") as String {format: "00"}
    var nSeconds = failIf( parts.seconds, (minutes) -> minutes < 0 or minutes > 59, "Field 'seconds': `$(parts.seconds)` must be between 0 and 59.") as String {format: "00"}
    ---
    "$(nYear)-$(nMonth)-$(nDay)T$(nHours):$(nMinutes):$(nSeconds)" as LocalDateTime
}



/**
* Creates a `LocalTime` value from values specified for `hour`, `minutes`, and
* `seconds` fields.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | parts | `Number` values for `hour`, `minutes`, and
*           `seconds` fields. Valid values are 0 through 23 for the
*           `hour`, 0 through 59 for `minutes`, and 0 through 59 (including decimals, such as 59.99) for `seconds` fields.
*           You can specify the name-value pairs in any order,
*           but the output is ordered as a default `LocalTime` value,
*           such as `10:10:10`. The input fields are parts of
*           a `LocalDateTimeFactory` type.
* |===
*
* === Example
*
* This example shows how to create a value of type `LocalTime`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*   newLocalTime: localTime({ hour: 12, minutes: 30, seconds: 40})
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "newLocalTime": "12:30:40"
* }
* ----
**/
@Since(version = "2.4.0")
fun localTime(parts: LocalTimeFactory): LocalTime = do {
    var nHours = failIf( parts.hour, (hour) -> hour < 0 or hour > 23, "Field 'hours': `$(parts.hour)` must be between 0 and 23.") as String {format: "00"}
    var nMinutes = failIf( parts.minutes, (minutes) -> minutes < 0 or minutes > 59, "Field 'minutes': `$(parts.minutes)` must be between 0 and 59.") as String {format: "00"}
    var nSeconds = failIf( parts.seconds, (minutes) -> minutes < 0 or minutes > 59, "Field 'seconds': `$(parts.seconds)` must be between 0 and 59.") as String {format: "00"}
    ---
    "$(nHours):$(nMinutes):$(nSeconds)" as LocalTime
}


/**
* Creates a `Time` value from values specified for `hour`, `minutes`, `seconds`, and
* `timezone` fields.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | parts | `Number` values for `hour`, `minutes`, and `seconds` fields, and a
*           `TimeZone` value for the `timezone` field. Valid values are 0
*           through 23 for the `hour`, 0 through 59 for `minutes`, and 0 through 59 (including decimals, such as 59.99) for `seconds` fields. The `timezone` must be a valid `TimeZone` value,
*           such as `&#124;-03:00&#124;` You can specify the name-value pairs in any
*           order, but the output is ordered as a default `Time` value,
*           such as `10:10:10-03:00`. The input fields are parts of
*           a `TimeFactory` type.
* |===
*
* === Example
*
* This example shows how to create a value of type `Time`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*   newTime: time({ hour: 12, minutes: 30, seconds: 40 , timeZone: |-03:00| })
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "newTime": "12:30:40-03:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun time(parts: TimeFactory): Time = do {
    var nHours = failIf( parts.hour, (hour) -> hour < 0 or hour > 23, "Field 'hours': `$(parts.hour)` must be between 0 and 23.") as String {format: "00"}
    var nMinutes = failIf( parts.minutes, (minutes) -> minutes < 0 or minutes > 59, "Field 'minutes': `$(parts.minutes)` must be between 0 and 59.") as String {format: "00"}
    var nSeconds = failIf( parts.seconds, (minutes) -> minutes < 0 or minutes > 59, "Field 'seconds': `$(parts.seconds)` must be between 0 and 59.") as String {format: "00"}
    ---
    "$(nHours):$(nMinutes):$(nSeconds)$(parts.timeZone as String)" as Time
}

/**
* Returns a  new `DateTime` value that changes the `Time` value in the input to the
* beginning of the specified _hour_.
*
*
* The minutes and seconds in the input change to `00:00`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | dateTime |  The `DateTime` value to reference.
* |===
*
* === Example
*
* This example changes the `Time` value within the `DateTime` input to the
* beginning of the specified _hour_.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*    "atBeginningOfHourDateTime": atBeginningOfHour(|2020-10-06T18:23:20.351-03:00|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*  "atBeginningOfHourDateTime": "2020-10-06T18:00:00-03:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfHour(dateTime: DateTime): DateTime =
  "$(dateTime as Date as String)T$(dateTime.hour as String {format: '00'}):00:00.000$(dateTime.timezone as String)" as DateTime


/**
* Returns a  new `LocalDateTime` value that changes the `Time` value in the input to the
* beginning of the specified _hour_.
*
*
* The minutes and seconds in the input change to `00:00`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | localDateTime | The `LocalDateTime` value to reference.
* |===
*
* === Example
*
* This example changes the `Time` value within the `LocalDateTime` input to the
* beginning of the specified hour.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*    "atBeginningOfHourLocalDateTime": atBeginningOfHour(|2020-10-06T18:23:20.351|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*  "atBeginningOfHourLocalDateTime": "2020-10-06T18:00:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfHour(localDateTime: LocalDateTime): LocalDateTime =
  "$(localDateTime as Date as String)T$(localDateTime.hour as String {format: '00'}):00:00.000" as LocalDateTime


/**
* Returns a  new `LocalTime` value that changes its value in the input to the
* beginning of the specified _hour_.
*
*
* The minutes and seconds in the input change to `00:00`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | localTime | The `LocalTime` value to reference.
* |===
*
* === Example
*
* This example changes the `LocalTime` value to the
* beginning of the specified hour.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*    "atBeginningOfHourLocalTime": atBeginningOfHour(|18:23:20.351|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*  "atBeginningOfHourLocalTime": "18:00:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfHour(localTime: LocalTime): LocalTime =
  "$(localTime.hour as String {format: '00'}):00:00.000" as LocalTime


/**
* Returns a new `Time` value that changes the input value to the
* beginning of the specified _hour_.
*
*
* The minutes and seconds in the input change to `00:00`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | time | The `Time` value to reference.
* |===
*
* === Example
*
* This example changes the `Time` value to the beginning of the specified hour.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*    "atBeginningOfHourTime": atBeginningOfHour(|18:23:20.351-03:00|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*  "atBeginningOfHourTime":  "18:00:00-03:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfHour(time: Time): Time =
  "$(time.hour as String {format: '00'}):00:00.000$(time.timezone as String)" as Time

/**
* Returns a  new `DateTime` value that changes the `Time` value in the input to the
* beginning of the specified _day_.
*
*
* The hours, minutes, and seconds in the input change to `00:00:00`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | dateTime | The `DateTime` value to reference.
* |===
*
* === Example
*
* This example changes the `Time` value within the `DateTime` input to the
* beginning of the specified day.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*   "atBeginningOfDayDateTime": atBeginningOfDay(|2020-10-06T18:23:20.351-03:00|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "atBeginningOfDayDateTime": "2020-10-06T00:00:00-03:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfDay(dateTime: DateTime): DateTime =
  "$(dateTime as Date as String)T00:00:00.000$(dateTime.timezone as String)" as DateTime


/**
* Returns a new `LocalDateTime` value that changes the `Time` value within the
* input to the start of the specified _day_.
*
*
* The hours, minutes, and seconds in the input change to `00:00:00`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | localDateTime | The `LocalDateTime` value to reference.
* |===
*
* === Example
*
* This example changes the `Time` value within the `LocalDateTime` input to the
* beginning of the specified day.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*   "atBeginningOfDayLocalDateTime": atBeginningOfDay(|2020-10-06T18:23:20.351|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "atBeginningOfDayLocalDateTime": "2020-10-06T00:00:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfDay(localDateTime: LocalDateTime): LocalDateTime =
  "$(localDateTime as Date as String)T00:00:00.000" as LocalDateTime


/**
* Returns a new `DateTime` value that changes the `Day` value from the
* input to the first day of the specified _month_. It also sets the `Time` value to `00:00:00`.
*
*
* The day and time in the input changes to `01T00:00:00`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | dateTime | The `DateTime` value to reference.
* |===
*
* === Example
*
* This example changes the `Day` value within the `DateTime` input to the
* first day of the specified month and sets the `Time` value to `00:00:00`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*   "atBeginningOfMonthDateTime": atBeginningOfMonth(|2020-10-06T18:23:20.351-03:00|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "atBeginningOfMonthDateTime": "2020-10-01T00:00:00-03:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfMonth(dateTime: DateTime): DateTime =
  "$((dateTime as Date - days(dateTime.day - 1)) as String)T00:00:00.000$(dateTime.timezone as String)" as DateTime



/**
* Returns a new `LocalDateTime` value that changes the `Day` and `LocalTime`
* values from the input to the beginning of the specified _month_.
*
*
* The day and time in the input changes to `01T00:00:00`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | localDateTime | The `LocalDateTime` value to reference.
* |===
*
* === Example
*
* This example changes the `Day` and `LocalTime` values within the `LocalDateTime`
* input to the beginning of the specified month.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*  "atBeginningOfMonthLocalDateTime": atBeginningOfMonth(|2020-10-06T18:23:20.351|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "atBeginningOfMonthLocalDateTime": "2020-10-01T00:00:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfMonth(localDateTime: LocalDateTime): LocalDateTime =
   "$((localDateTime as Date - days(localDateTime.day - 1)) as String)T00:00:00.000" as LocalDateTime



/**
* Returns a new `Date` value that changes the `Day` value from the
* input to the first day of the specified _month_.
*
*
* The day in the input changes to `01`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | date | The `Date` value to reference.
* |===
*
* === Example
*
* This example changes the `Day` value within the `Date`
* input to the first day of the specified month.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*   atBeginningOfMonthDate: atBeginningOfMonth(|2020-10-06|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "atBeginningOfMonthDate": "2020-10-01"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfMonth(date: Date): Date =
    "$((date - days(date.day - 1)) as String)" as Date

/**
* Returns a new `DateTime` value that changes the `Day` and `Time` values from the
* input to the beginning of the first day of the specified _week_.
*
*
* The function treats Sunday as the first day of the week.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | dateTime | The `DateTime` value to reference.
* |===
*
* === Example
*
* This example changes the `Day` and `Time` values (`06T18:23:20.351`) within
* the `DateTime` input to the beginning of the first day of the specified _week_
* (`04T00:00:00`).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*   atBeginningOfWeekDateTime: atBeginningOfWeek(|2020-10-06T18:23:20.351-03:00|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "atBeginningOfWeekDateTime": "2020-10-04T00:00:00-03:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfWeek(dateTime: DateTime): DateTime = do {
  var modDay = mod(dateTime.dayOfWeek, 7)
  ---
  "$((dateTime as Date - days(modDay)) as String)T00:00:00.000$(dateTime.timezone as String)" as DateTime
}
  

/**
* Returns a new `LocalDateTime` value that changes the `Day` and `Time` values from the
* input to the beginning of the first day of the specified _week_.
*
*
* The function treats Sunday as the first day of the week.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | localDateTime | The `LocalDateTime` value to reference.
* |===
*
* === Example
*
* This example changes the `Day` and `Time` values (`06T18:23:20.351`) within
* the `LocalDateTime` input to the beginning of the first day of the specified _week_
* (`04T00:00:00`).
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*   atBeginningOfWeekLocalDateTime: atBeginningOfWeek(|2020-10-06T18:23:20.351|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "atBeginningOfWeekLocalDateTime": "2020-10-04T00:00:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfWeek(localDateTime: LocalDateTime): LocalDateTime = do {
  var modDay = mod(localDateTime.dayOfWeek, 7)
  ---
  "$((localDateTime as Date - days(modDay)) as String)T00:00:00.000" as LocalDateTime
}


/**
* Returns a new `Date` value that changes the `Date` input
* input to the first day of the specified _week_.
*
*
* The function treats Sunday as the first day of the week.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | date | The `Date` value to reference.
* |===
*
* === Example
*
* This example changes the `Day` value (`06`) within
* the `Date` input to the first day of the week that contains `2020-10-06` (a Tuesday), which is `2020-10-04` (a Sunday).
* The `Day` value changes from `06` to `04`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*   atBeginningOfWeekDate: atBeginningOfWeek(|2020-10-06|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "atBeginningOfWeekDate": "2020-10-04"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfWeek(date: Date): Date = do {
  var modDay = mod(date.dayOfWeek, 7)
  ---
  "$((date - days(modDay)) as String)" as Date
}
  


/**
* Takes a `DateTime` value as input and returns a `DateTime` value for
* the first day of the _year_ specified in the input. It also sets the `Time` value to `00:00:00`.
*
*
* The month, day, and time in the input changes to `01-01T00:00:00`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | dateTime | The `DateTime` value to reference.
* |===
*
* === Example
*
* This example transforms the `DateTime` input (`&#124;2020-10-06T18:23:20.351-03:00&#124;`)
* to the date of the first day of the `Year` value (`2020`) in the input.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*   atBeginningOfYearDateTime: atBeginningOfYear(|2020-10-06T18:23:20.351-03:00|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "atBeginningOfYearDateTime": "2020-01-01T00:00:00.000-03:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfYear(dateTime: DateTime): DateTime =
  "$((dateTime as Date - days(dateTime.dayOfYear - 1)) as String)T00:00:00.000$(dateTime.timezone as String)" as DateTime


/**
* Takes a `LocalDateTime` value as input and returns a `LocalDateTime` value for
* the first day of the _year_ specified in the input. It also sets the `Time` value to `00:00:00`.
*
*
* The month, day, and time in the input changes to `01-01T00:00:00`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | localDateTime | The `LocalDateTime` value to reference.
* |===
*
* === Example
*
* This example transforms the `LocalDateTime` input (`|2020-10-06T18:23:20.351|`)
* to the date of the first day of the `Year` value (`2020`) in the input.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*   atBeginningOfYearLocalDateTime: atBeginningOfYear(|2020-10-06T18:23:20.351|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "atBeginningOfYearLocalDateTime": "2020-01-01T00:00:00"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfYear(localDateTime: LocalDateTime): LocalDateTime =
  "$((localDateTime as Date - days(localDateTime.dayOfYear - 1)) as String)T00:00:00.000" as LocalDateTime



/**
* Takes a `Date` value as input and returns a `Date` value for
* the first day of the _year_ specified in the input.
*
*
* The month and day in the input changes to `01-01`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | date | The `Date` value to reference.
* |===
*
* === Example
*
* This example transforms `Date` input (`|2020-10-06|`) to the date of the
* first day of the `Year` value (`2020`) in the input.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::core::Dates
* output application/json
* ---
* {
*   atBeginningOfYearDate: atBeginningOfYear(|2020-10-06|)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "atBeginningOfYearDate": "2020-01-01"
* }
* ----
**/
@Since(version = "2.4.0")
fun atBeginningOfYear(date: Date): Date =
  "$((date as Date - days(date.dayOfYear - 1)) as String)" as Date
