/**
* A utility module that provides mathematical functions.
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::util::Math` to the header of your
* DataWeave script.
*/
@Since(version = "2.4.0")
%dw 2.0

/**
* Variable `E` sets the value of mathematical constant `e`,
* the base of natural logarithms.
*/
@Since(version = "2.4.0")
var E = 2.7182818284590452354

/**
* Variable `PI` sets the value of constant value pi, the ratio
* of the circumference of a circle to its diameter.
*/
@Since(version = "2.4.0")
var PI = 3.14159265358979323846

/**
* Returns the trigonometric sine of an angle from a given number of radians.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | angle | Number of radians in an angle.
* |===
*
* === Example
*
* This example shows how `sin` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Math
* output application/json
* ---
* {
*   "sin0": sin(0),
*   "sin13": sin(0.13),
*   "sin-1": sin(-1)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
*{
*   "sin0": 0.0,
*   "sin13": 0.12963414261969486,
*   "sin-1": -0.8414709848078965
* }
* ----
**/
@Since(version = "2.4.0")
fun sin(angle: Number):Number = native("system::SinFunctionValue")

/**
* Returns the trigonometric cosine of an angle from a given number of radians.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | angle | Number of radians in an angle.
* |===
*
* === Example
*
* This example shows how `cos` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Math
* output application/json
* ---
* {
*   "cos0": cos(0),
*   "cos13": cos(0.13),
*   "cos-1": cos(-1)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*   "cos0": 1.0,
*   "cos13": 0.9915618937147881,
*   "cos-1": 0.5403023058681398
* }
* ----
**/
@Since(version = "2.4.0")
fun cos(angle: Number):Number = native("system::CosFunctionValue")

/**
* Returns the trigonometric tangent of an angle from a given number of radians.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | angle | Number of radians in an angle.
* |===
*
* === Example
*
* This example shows how `tan` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Math
* output application/json
* ---
* {
*    "tan0": tan(0),
*    "tan13": tan(0.13),
*    "tan-1": tan(-1)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "tan0": 0.0,
*    "tan13": 0.13073731800446006,
*    "tan-1": -1.5574077246549023
*  }
* ----
**/
@Since(version = "2.4.0")
fun tan(angle: Number):Number = native("system::TanFunctionValue")

/**
* Returns an arc sine value that can range from `-pi/2` through `pi/2`.
*
*
* If the absolute value of the input is greater than 1, the result
* is `null`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | angle | Number to convert into its arc sine value.
* |===
*
* === Example
*
* This example shows how `asin` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Math
* output application/json
* ---
* {
*   "asin0": asin(0),
*   "asin13": asin(0.13),
*   "asin-1": asin(-1),
*   "asin1.1": asin(1.1)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "asin0": 0.0,
*    "asin13": 0.1303689797031455,
*    "asin-1": -1.5707963267948966,
*    "asin1.1": null
*  }
* ----
**/
@Since(version = "2.4.0")
fun asin(angle: Number): Number | NaN = native("system::ASinFunctionValue")

/**
* Returns an arc cosine value that can range from `0.0` through pi.
*
*
* If the absolute value of the input is greater than `1`,
* the result is `null`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | angle | Number to convert into it arc cosine value.
* |===
*
* === Example
*
* This example shows how `acos` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Math
* output application/json
* ---
* {
*   "acos0": acos(0),
*   "acos13": acos(0.13),
*   "acos-1": acos(-1),
*   "acos1": acos(1),
*   "acos1.1": acos(1.1)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "acos0": 1.5707963267948966,
*    "acos13": 1.440427347091751,
*    "acos-1": 3.141592653589793,
*    "acos1": 0.0,
*    "acos1.1": null
*  }
* ----
**/
@Since(version = "2.4.0")
fun acos(angle: Number):Number | NaN = native("system::ACosFunctionValue")

/**
* Returns an arc tangent value that can range from `-pi/2` through `pi/2`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | angle | Number to convert into its arc tangent value.
* |===
*
* === Example
*
* This example shows how `atan` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Math
* output application/json
* ---
* {
*   "atan0":  atan(0),
*   "atan13": atan(0.13),
*   "atan-1": atan(-1)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "atan0": 0.0,
*    "atan13": 0.12927500404814307,
*    "atan-1": -0.7853981633974483
* }
* ----
**/
@Since(version = "2.4.0")
fun atan(angle: Number):Number = native("system::ATanFunctionValue")

/**
* Converts a given number of degrees in an angle to an approximately
* equivalent number of radians.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | angdeg | Number of degrees to convert into radians.
* |===
*
* === Example
*
* This example shows how `toRadians` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Math
* output application/json
* ---
* {
*   "toRadians10":  toRadians(10),
*   "toRadians013": toRadians(0.13),
*   "toRadians-20": toRadians(-20)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "toRadians10": 0.1745329251994329576922222222222222,
*    "toRadians013": 0.002268928027592628449998888888888889,
*    "toRadians-20": -0.3490658503988659153844444444444444
*  }
* ----
**/
@Since(version = "2.4.0")
fun toRadians(angdeg:Number): Number = angdeg / 180.0 * PI

/**
* Converts an angle measured in radians to an approximately
* equivalent number of degrees.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | angrad | Number of radians to convert to degrees.
* |===
*
* === Example
*
* This example shows how `toDegrees` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Math
* output application/json
* ---
* {
*   "toDegrees0.17":  toDegrees(0.174),
*   "toDegrees0": toDegrees(0),
*   "toDegrees-20": toDegrees(-0.20)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "toDegrees0.17": 9.969465635276323832571267395889251,
*    "toDegrees0": 0E+19,
*    "toDegrees-20": -11.45915590261646417536927286883822
*  }
* ----
**/
@Since(version = "2.4.0")
fun toDegrees(angrad:Number):Number = angrad * 180.0 / PI

/**
*  Returns the natural logarithm (base `e`) of a number.
*
*
* If the input value is less than or equal to zero,
* the result is `NaN` (or `null`).
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | a | Number to convert into its natural logarithm.
* |===
*
* === Example
*
* This example shows how `logn` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Math
* output application/json
* ---
* {
*    "logn10":  logn(10),
*    "logn13": logn(0.13),
*    "logn-20": logn(-20)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "logn10": 2.302585092994046,
*    "logn13": -2.0402208285265546,
*    "logn-20": null
* }
* ----
**/
@Since(version = "2.4.0")
fun logn(a: Number): Number | NaN = native("system::LognFunctionValue")

/**
* Returns the logarithm base 10 of a number.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name   | Description
* | a | A `Number` value that serves as input to the function.
* |===
*
* === Example
*
* This example shows how `log10` behaves with different inputs.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::util::Math
* output application/json
* ---
* {
*   "log1010": log10(10),
*   "log1013": log10(0.13),
*   "log10-20": log10(-20)
* }
* ----
*
* ==== Output
*
* [source,Json,linenums]
* ----
* {
*    "log1010": 1.0,
*    "log1013": -0.8860566476931632,
*    "log10-20": null
* }
* ----
**/
@Since(version = "2.4.0")
fun log10(a: Number): Number | NaN = native("system::Log10FunctionValue")

/**
* Rounding mode to round away from zero. Always increments the digit prior to a nonzero discarded fraction.
*/
type RoundUp = "UP"

/**
* Rounding mode to round towards zero. Never increments the digit prior to a discarded fraction (i.e., truncates).
*/
type RoundDown = "DOWN"

/**
* Rounding mode to round towards positive infinity. If the number is positive, behaves as for ROUND_UP; if negative, behaves as for ROUND_DOWN.
* Note that this rounding mode never decreases the calculated value.
*/
type RoundCeiling = "CEILING"

/**
* Rounding mode to round towards negative infinity. If the number is positive, behave as for ROUND_DOWN; if negative, behave as for ROUND_UP.
*/
type RoundFloor = "FLOOR"

/**
* Rounding mode to round towards "nearest neighbor" unless both neighbors are equidistant, in which case round up.
* Behaves as for ROUND_UP if the discarded fraction is ≥ 0.5; otherwise, behaves as for ROUND_DOWN.
*/
type RoundHalfUp = "HALF_UP"

/**
* Rounding mode to round towards "nearest neighbor" unless both neighbors are equidistant, in which case round down.
* Behaves as for ROUND_UP if the discarded fraction is > 0.5; otherwise, behaves as for ROUND_DOWN.
*/
type RoundHalfDown = "HALF_DOWN"

/**
* Rounding mode to round towards the "nearest neighbor" unless both neighbors are equidistant, in which case, round towards the even neighbor.
* Behaves as for ROUND_HALF_UP if the digit to the left of the discarded fraction is odd; behaves as for ROUND_HALF_DOWN if it's even.
*/
type RoundHalfEven = "HALF_EVEN"

/**
* Rounding mode to assert that the requested operation has an exact result, hence no rounding is necessary.
*/
type RoundUnnecessary = "UNNECESSARY"

type RoundingMode = RoundUp | RoundDown | RoundCeiling | RoundFloor | RoundHalfUp | RoundHalfDown | RoundHalfEven | RoundUnnecessary

type OperationContext = {|
  precision?: Number,
  roundingMode?: RoundingMode
|}

/**
* A MathContext object with a precision setting matching the precision of the IEEE 754-2019 decimal128 format, 34 digits, and a rounding mode of HALF_EVEN.
*/
var DECIMAL_128_CONTEXT: OperationContext = {
  precision: 34,
  roundingMode: "HALF_EVEN"
}

/**
* A MathContext object whose settings have the values required for unlimited precision arithmetic.
* The values of the settings are: precision=0 roundingMode=HALF_UP
*/
var UNLIMITED_CONTEXT: OperationContext = {
  precision: 0,
  roundingMode: "HALF_UP"
}

/**
* Performs number addition with rounding specified by the operation context
*/
fun decimalAdd(lhs: Number, rhs: Number, ctx: OperationContext = DECIMAL_128_CONTEXT): Number = native("system::BigDecimalAdditionFunctionValue")

/**
* Performs number subtraction with rounding specified by the operation context
*/
fun decimalSubtract(lhs: Number, rhs: Number, ctx: OperationContext = DECIMAL_128_CONTEXT): Number = native("system::BigDecimalSubtractionFunctionValue")

/**
* Performs number division with rounding specified by the operation context. If precision is set to 0 (unlimited precision)
* and the result has an infinite decimal expansion it will error
*/
fun decimalDivide(dividend: Number, divisor: Number, ctx: OperationContext = DECIMAL_128_CONTEXT): Number = native("system::BigDecimalDivisionFunctionValue")

/**
* Performs number multiplication with rounding specified by the operation context
*/
fun decimalMultiply(leftFactor: Number, rightFactor: Number, ctx: OperationContext = DECIMAL_128_CONTEXT): Number = native("system::BigDecimalMultiplicationFunctionValue")

/**
* Returns a number with value base^exponent with rounding specified by the operation context. Maximum value for exponent
* is 99999999.
*/
fun decimalPow(base: Number, exponent: Number, ctx: OperationContext = DECIMAL_128_CONTEXT): Number = native("system::BigDecimalPowerFunctionValue")

/**
* Returns a number that approximates the square root of the argument with rounding specified by the operation context
*/
fun decimalSqrt(n: Number, ctx: OperationContext = DECIMAL_128_CONTEXT): Number = native("system::BigDecimalSqrtFunctionValue")

/**
* Returns the argument number with rounding specified by the operation context
*/
fun decimalRound(n: Number, ctx: OperationContext = DECIMAL_128_CONTEXT) = native("system::BigDecimalRoundFunctionValue")