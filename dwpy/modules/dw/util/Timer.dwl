/**
* This utility module contains functions for measuring time.
*
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::util::Timer` to the header of your
* DataWeave script.
*/
%dw 2.0

// A type: TimeMeasurement.
/**
* A return type that contains a start time, end time, and result of a function call.
*/
type TimeMeasurement<T> =  {start: DateTime, result: T, end: DateTime}

// A type: DurationMeasurement.
/**
* A return type that contains the execution time and result of a function call.
*/
type DurationMeasurement<T> = {time: Number, result: T}

/**
* Returns the current time in milliseconds.
*
* === Example
*
* This example shows the time in milliseconds when the function executed.
*
* ==== Source
*
* [source,Dataweave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Timer
* output application/json
* ---
* { "currentMilliseconds" : currentMilliseconds() }
* ----
*
* ==== Output
*
* [source,XML,linenums]
* ----
* { "currentMilliseconds": 1532923168900 }
* ----
*/
fun currentMilliseconds(): Number =
    toMilliseconds(now())

/**
* Returns the representation of a specified date-time in milliseconds.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | date | A `DateTime` to evaluate.
* |===
*
* === Example
*
* This example shows a date-time in milliseconds.
*
* ==== Source
*
* [source,Dataweave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Timer
* output application/json
* ---
* { "toMilliseconds" : toMilliseconds(|2018-07-23T22:03:04.829Z|) }
* ----
*
* ==== Output
*
* [source,XML,linenums]
* ----
* { "toMilliseconds": 1532383384829 }
* ----
*/
fun toMilliseconds(date:DateTime): Number =
    date as Number {unit: "milliseconds"}

/**
* Executes the input function and returns an object with execution time in
* milliseconds and result of that function.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | valueToMeasure | A function to pass to `duration`.
* |===
*
* === Example
*
* This example passes a `wait` function (defined in the header), which returns
* the execution time and result of that function in a `DurationMeasurement`
* object.
*
* ==== Source
*
* [source,Dataweave,linenums]
* ----
* %dw 2.0
* output application/json
* fun myFunction() = dw::Runtime::wait("My result",100)
* ---
* dw::util::Timer::duration(() -> myFunction())
* ----
*
* ==== Output
*
* [source,XML,linenums]
* ----
* {
*   "time": 101,
*   "result": "My result"
* }
* ----
*/
fun duration<T>(valueToMeasure: ()-> T): DurationMeasurement<T> = do {
    var timeResult = time(valueToMeasure)
    ---
    {
        time: toMilliseconds(timeResult.end) - toMilliseconds(timeResult.start),
        result: timeResult.result
    }
}

/**
* Executes the input function and returns a `TimeMeasurement` object that
* contains the start and end time for the execution of that function, as well
* the result of the function.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | valueToMeasure | A function to pass to `time`.
* |===
*
* === Example
*
* This example passes `wait` and `sum` functions (defined in the
* header), which return their results in `TimeMeasurement`
* objects.
*
* [source,Dataweave, linenums]
* ----
* %dw 2.0
* output application/json
* fun myFunction() = dw::Runtime::wait("My result",100)
* fun myFunction2() = sum([1,2,3,4])
* ---
* { testing: [
*     dw::util::Timer::time(() -> myFunction()),
*     dw::util::Timer::time(() -> myFunction2())
*   ]
* }
* ----
*
* ==== Output
*
* [source,XML,linenums]
* ----
* {
*   "testing": [
*     {
*       "start": "2018-10-05T19:23:01.49Z",
*       "result": "My result",
*       "end": "2018-10-05T19:23:01.591Z"
*     },
*     {
*       "start": "2018-10-05T19:23:01.591Z",
*       "result": 10,
*       "end": "2018-10-05T19:23:01.591Z"
*     }
*   ]
* }
* ----
*/
fun time<T>(valueToMeasure: ()-> T): TimeMeasurement<T> = do {
    var statTime = now()
    var result = valueToMeasure()
    var endTime = now()
    ---
    {
        start: statTime,
        result: result,
        end: endTime
    }

}
