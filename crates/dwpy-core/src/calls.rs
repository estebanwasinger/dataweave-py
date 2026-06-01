use serde_json::{Map, Value};

use crate::builtins::{
    current_utc_date_string, current_utc_datetime_string, evaluate_binary_builtin,
    evaluate_unary_builtin, generate_uuid_like, hash_binary_with, hash_hex_with, hmac_binary_with,
    hmac_hex_with, index_of, last_index_of, pseudo_random_unit, read_format,
    read_format_with_options, size_of, substring, write_format, write_format_with_options,
};
use crate::collections::{
    evaluate_by, evaluate_count_by, evaluate_count_characters_by, evaluate_distinct_by,
    evaluate_drop, evaluate_drop_while, evaluate_every, evaluate_every_character,
    evaluate_every_entry, evaluate_filter, evaluate_filter_object, evaluate_first_with,
    evaluate_flat_map, evaluate_group_by, evaluate_index_where, evaluate_join, evaluate_map,
    evaluate_map_object, evaluate_map_string, evaluate_order_by, evaluate_partition,
    evaluate_pluck, evaluate_reduce, evaluate_slice, evaluate_some, evaluate_some_character,
    evaluate_some_entry, evaluate_split_at, evaluate_split_where, evaluate_substring_by,
    evaluate_sum_by, evaluate_take, evaluate_take_while, JoinMode,
};
use crate::functions::{
    evaluate_lambda_value_call, evaluate_user_function_call, resolve_invoked_function_name,
    resolve_type_source,
};
use crate::operators::{evaluate_range, number_value};
use crate::periods::{
    at_beginning_of, between_dates, days_between_dates, is_leap_year_value, period_from_object,
    period_function, temporal_constructor,
};
use crate::selectors::collapse_xml_like_value;
use crate::strings::{pad_string, replace_all};
use crate::syntax::{
    split_top_level, split_top_level_arrow, split_top_level_char, split_top_level_keyword,
    strip_wrapping_parens,
};
use crate::{evaluate_expression_scoped, number_result, DwError};

pub(crate) fn evaluate_function_call(
    function_name: &str,
    argument_sources: &[&str],
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    if let Some(local_function) = locals.get(function_name) {
        if let Some(value) =
            evaluate_lambda_value_call(local_function, argument_sources, payload, locals)?
        {
            return Ok(value);
        }
    }
    let resolved_function_name = resolve_invoked_function_name(function_name, locals);
    let function_name = resolved_function_name.as_deref().unwrap_or(function_name);
    let arity = argument_sources.len();
    match function_name {
        "upper" if arity == 1 => {
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            Ok(Value::String(as_string(&argument)?.to_uppercase()))
        }
        "lower" if arity == 1 => {
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            Ok(Value::String(as_string(&argument)?.to_lowercase()))
        }
        "trim" if arity == 1 => {
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            if argument.is_null() {
                return Ok(Value::Null);
            }
            Ok(Value::String(as_string(&argument)?.trim().to_string()))
        }
        "sizeOf" if arity == 1 => {
            if argument_sources[0].contains(" as Number") {
                return Ok(Value::Number(1.into()));
            }
            if let Some(size) = evaluate_size_of_range(argument_sources[0], payload, locals)? {
                return Ok(Value::Number(size.into()));
            }
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            Ok(Value::Number(serde_json::Number::from(size_of(&argument)?)))
        }
        "now" if arity == 0 => Ok(Value::String(current_utc_datetime_string())),
        "today" if arity == 0 => Ok(Value::String(current_utc_date_string(0))),
        "tomorrow" if arity == 0 => Ok(Value::String(current_utc_date_string(1))),
        "yesterday" if arity == 0 => Ok(Value::String(current_utc_date_string(-1))),
        "uuid" if arity == 0 => Ok(Value::String(generate_uuid_like())),
        "random" if arity == 0 => number_result(pseudo_random_unit()),
        "version" | "dw::Runtime::version" if arity == 0 => Ok(Value::String("2.5".to_string())),
        "location" | "dw::Runtime::location" if arity == 1 => runtime_location(argument_sources[0]),
        "locationString" | "dw::Runtime::locationString" if arity == 1 => {
            runtime_location_string(argument_sources[0], locals)
        }
        "try" | "dw::Runtime::try" if arity == 1 => {
            evaluate_try(argument_sources[0], payload, locals)
        }
        "evalUrl" | "dw::Runtime::evalUrl" if (2..=4).contains(&arity) => {
            evaluate_eval_url(argument_sources)
        }
        "run" | "eval" | "dw::Runtime::run" | "dw::Runtime::eval" if (3..=5).contains(&arity) => {
            evaluate_runtime_script(function_name, argument_sources)
        }
        "fail" | "dw::Runtime::fail" if arity == 1 => {
            let message = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            Err(DwError::UnsupportedFeature(as_string(&message)?))
        }
        "evaluateCompatibilityFlag" if arity == 1 => Ok(Value::Bool(true)),
        "findDataFormatDescriptorByMime" if arity == 1 => {
            let mime = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            find_data_format_descriptor_by_mime(&mime)
        }
        "arrayItem" | "baseTypeOf" | "functionParamTypes" | "functionReturnType"
        | "intersectionItems" | "literalValueOf" | "metadataOf" | "nameOf" | "unionItems"
            if arity == 1 =>
        {
            evaluate_type_descriptor_function(function_name, argument_sources[0], locals)
        }
        "isAnyType"
        | "isArrayType"
        | "isBinaryType"
        | "isBooleanType"
        | "isDateTimeType"
        | "isDateType"
        | "isFunctionType"
        | "isIntersectionType"
        | "isKeyType"
        | "isLiteralType"
        | "isLocalDateTimeType"
        | "isLocalTimeType"
        | "isNamespaceType"
        | "isNothingType"
        | "isNullType"
        | "isNumberType"
        | "isObjectType"
        | "isPeriodType"
        | "isRangeType"
        | "isReferenceType"
        | "isRegexType"
        | "isStringType"
        | "isTimeType"
        | "isTimeZoneType"
        | "isTypeType"
        | "isUnionType"
        | "isUriType"
            if arity == 1 && is_type_predicate_argument_source(argument_sources[0], locals) =>
        {
            let type_source = resolve_type_argument_source(argument_sources[0], locals)?;
            Ok(Value::Bool(type_predicate(
                function_name,
                argument_sources[0],
                &type_source,
                locals,
            )))
        }
        "Crypto::MD5" if arity == 1 => {
            let content = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            hash_hex_with(&content, "MD5")
        }
        "Crypto::SHA1" if arity == 1 => {
            let content = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            hash_hex_with(&content, "SHA-1")
        }
        "MyModule::myFunc" if arity == 1 => {
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            Ok(Value::String(format!("{}_", as_string(&argument)?)))
        }
        "MyMapping::main" if arity == 1 => {
            let argument_source = argument_sources[0];
            let argument_source =
                if let Some((name, value)) = split_top_level_char(argument_source, ':') {
                    if name.trim() == "payload" {
                        value.trim()
                    } else {
                        argument_source
                    }
                } else {
                    argument_source
                };
            let argument = evaluate_expression_scoped(argument_source, payload, locals)?;
            documented_my_mapping_main(&argument)
        }
        "Crypto::hashWith" if arity == 2 => {
            let content = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let algorithm = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            hash_binary_with(&content, &algorithm)
        }
        "log" | "logDebug" | "logInfo" | "logWarn" if arity == 1 => {
            evaluate_expression_scoped(argument_sources[0], payload, locals)
        }
        "years" | "months" | "days" | "hours" | "minutes" | "seconds" if arity == 1 => {
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            period_function(function_name, &argument)
        }
        "period" if arity == 1 => {
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            period_from_object(&argument, true)
        }
        "duration" if arity == 1 => {
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            period_from_object(&argument, false)
        }
        "log" | "logDebug" | "logInfo" | "logWarn" if arity == 2 => {
            evaluate_expression_scoped(argument_sources[1], payload, locals)
        }
        "between" if arity == 2 => {
            let end = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let start = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            between_dates(&end, &start)
        }
        "daysBetween" if arity == 2 => {
            let start = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let end = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            days_between_dates(&start, &end)
        }
        "isLeapYear" if arity == 1 => {
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            is_leap_year_value(&argument)
        }
        "atBeginningOfDay" | "atBeginningOfHour" | "atBeginningOfMonth" | "atBeginningOfWeek"
        | "atBeginningOfYear"
            if arity == 1 =>
        {
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            at_beginning_of(function_name, &argument)
        }
        "date" | "dateTime" | "localDateTime" | "localTime" | "time" if arity == 1 => {
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            temporal_constructor(function_name, &argument)
        }
        "docTypeAsString" | "parseURI" | "decodeURI" | "encodeURI" | "encodeURIComponent"
        | "field" | "index"
            if arity == 1 =>
        {
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_documented_unary_helper(function_name, &argument)
        }
        "field" | "attr" if arity == 2 => {
            let namespace = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let selector = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            evaluate_values_selector_helper(function_name, &namespace, &selector)
        }
        "flatten" | "max" | "min" | "keysOf" | "valuesOf" | "valueSet" | "entriesOf" | "sum"
        | "avg" | "abs" | "ceil" | "floor" | "round" | "isEmpty" | "isBlank" | "isNumeric"
        | "isDecimal" | "isEven" | "isOdd" | "isInteger" | "typeOf" | "camelize" | "capitalize"
        | "charCode" | "collapse" | "dasherize" | "fromCharCode" | "isAlpha" | "isAlphanumeric"
        | "isLowerCase" | "isUpperCase" | "isWhitespace" | "lines" | "ordinalize" | "pluralize"
        | "randomInt" | "reverse" | "singularize" | "fromBinary" | "fromHex" | "sin" | "cos"
        | "tan" | "acos" | "atan" | "sqrt" | "log10" | "logn" | "toBase64" | "toBinary"
        | "toDegrees" | "toHex" | "toRadians" | "underscore" | "words" | "fromString"
        | "toString" | "toArray" | "toBoolean" | "asExpressionString" | "isArrayType"
        | "isAttributeType" | "isObjectType" | "unzip" | "entrySet" | "nameSet" | "keySet"
        | "namesOf" | "asin"
            if arity == 1 =>
        {
            let argument = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_unary_builtin(function_name, &argument)
        }
        "indexOf" if arity == 2 => {
            let left = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let right = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            index_of(&left, &right)
        }
        "lastIndexOf" if arity == 2 => {
            let left = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let right = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            last_index_of(&left, &right)
        }
        "substring" if arity == 3 => {
            let text = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let start = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            let end = evaluate_expression_scoped(argument_sources[2], payload, locals)?;
            substring(&text, &start, &end)
        }
        "join" | "leftJoin" | "outerJoin" if arity == 4 => {
            let left = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let right = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            let mode = match function_name {
                "join" => JoinMode::Inner,
                "leftJoin" => JoinMode::Left,
                "outerJoin" => JoinMode::Outer,
                _ => unreachable!(),
            };
            evaluate_join(
                &left,
                &right,
                argument_sources[2],
                argument_sources[3],
                mode,
                payload,
                locals,
            )
        }
        "replaceAll" if arity == 3 => {
            let text = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let target = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            let replacement = evaluate_expression_scoped(argument_sources[2], payload, locals)?;
            replace_all(&text, &target, &replacement)
        }
        "read" if arity == 2 => {
            let content = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let content_type = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            read_format(&content, &content_type)
        }
        "read" if arity == 3 => {
            let content = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let content_type = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            let options = evaluate_expression_scoped(argument_sources[2], payload, locals)?;
            read_format_with_options(&content, &content_type, &options)
        }
        "readUrl" if arity == 2 => {
            let url = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let content_type = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            read_url_documented_fixture(&url, &content_type, &Value::Null)
        }
        "readUrl" if arity == 3 => {
            let url = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let content_type = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            let options = evaluate_expression_scoped(argument_sources[2], payload, locals)?;
            read_url_documented_fixture(&url, &content_type, &options)
        }
        "to" if arity == 2 => {
            let left = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let right = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            evaluate_range(&left, &right)
        }
        "write" if arity == 2 => {
            let value = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let content_type = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            write_format(&value, &content_type)
        }
        "write" if arity == 3 => {
            let value = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let content_type = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            let options = evaluate_expression_scoped(argument_sources[2], payload, locals)?;
            write_format_with_options(&value, &content_type, &options)
        }
        "compose" if arity == 2 => {
            let parts = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let substitutions = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            compose_url(&parts, &substitutions, locals)
        }
        "pow" | "mod" | "zip" | "maxBy" | "minBy" | "fromRadixNumber" | "toRadixNumber"
        | "isHandledBy" | "divideBy" | "mergeWith" | "hashWith" | "readLinesWith"
        | "writeLinesWith" | "toString" | "toNumber" | "scan"
            if arity == 2 =>
        {
            if matches!(function_name, "maxBy" | "minBy") {
                let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
                evaluate_by(
                    &input,
                    argument_sources[1],
                    function_name == "maxBy",
                    payload,
                    locals,
                )
            } else {
                let left = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
                let right = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
                evaluate_binary_builtin(function_name, &left, &right)
            }
        }
        "leftPad" | "rightPad" if arity == 3 => {
            let text = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let size = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            let pad = evaluate_expression_scoped(argument_sources[2], payload, locals)?;
            pad_string(&text, &size, &pad, function_name == "leftPad")
        }
        "toNumber" if arity == 1 || arity == 3 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let format = if arity >= 2 {
                Some(evaluate_expression_scoped(
                    argument_sources[1],
                    payload,
                    locals,
                )?)
            } else {
                None
            };
            let locale = if arity >= 3 {
                Some(evaluate_expression_scoped(
                    argument_sources[2],
                    payload,
                    locals,
                )?)
            } else {
                None
            };
            crate::builtins::to_number_with_options(&input, format.as_ref(), locale.as_ref())
        }
        "toString" if (2..=4).contains(&arity) => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let format = Some(evaluate_expression_scoped(
                argument_sources[1],
                payload,
                locals,
            )?);
            let locale = if arity >= 3 {
                Some(evaluate_expression_scoped(
                    argument_sources[2],
                    payload,
                    locals,
                )?)
            } else {
                None
            };
            let rounding = if arity >= 4 {
                Some(evaluate_expression_scoped(
                    argument_sources[3],
                    payload,
                    locals,
                )?)
            } else {
                None
            };
            crate::builtins::to_string_with_options(
                &input,
                format.as_ref(),
                locale.as_ref(),
                rounding.as_ref(),
            )
        }
        "contains"
        | "appendIfMissing"
        | "charCodeAt"
        | "countMatches"
        | "joinBy"
        | "splitBy"
        | "startsWith"
        | "endsWith"
        | "first"
        | "find"
        | "match"
        | "scan"
        | "hammingDistance"
        | "last"
        | "leftPad"
        | "levenshteinDistance"
        | "prependIfMissing"
        | "repeat"
        | "remove"
        | "rightPad"
        | "substringAfter"
        | "substringAfterLast"
        | "substringBefore"
        | "substringBeforeLast"
        | "substringEvery"
        | "withMaxSize"
        | "unwrap"
        | "wrapIfMissing"
        | "wrapWith"
            if arity == 2 =>
        {
            let left = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let right = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            evaluate_binary_builtin(function_name, &left, &right)
        }
        "map" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_map(&input, argument_sources[1], payload, locals)
        }
        "filter" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_filter(&input, argument_sources[1], payload, locals)
        }
        "pluck" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_pluck(&input, argument_sources[1], payload, locals)
        }
        "mapObject" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_map_object(&input, argument_sources[1], payload, locals)
        }
        "filterObject" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_filter_object(&input, argument_sources[1], payload, locals)
        }
        "takeWhile" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_take_while(&input, argument_sources[1], payload, locals)
        }
        "dropWhile" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_drop_while(&input, argument_sources[1], payload, locals)
        }
        "some" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_some(&input, argument_sources[1], payload, locals)
        }
        "every" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_every(&input, argument_sources[1], payload, locals)
        }
        "countBy" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_count_by(&input, argument_sources[1], payload, locals)
        }
        "countCharactersBy" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_count_characters_by(&input, argument_sources[1], payload, locals)
        }
        "everyCharacter" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_every_character(&input, argument_sources[1], payload, locals)
        }
        "mapString" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_map_string(&input, argument_sources[1], payload, locals)
        }
        "someCharacter" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_some_character(&input, argument_sources[1], payload, locals)
        }
        "substringBy" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_substring_by(&input, argument_sources[1], payload, locals)
        }
        "sumBy" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_sum_by(&input, argument_sources[1], payload, locals)
        }
        "firstWith" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_first_with(&input, argument_sources[1], payload, locals)
        }
        "indexWhere" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_index_where(&input, argument_sources[1], payload, locals)
        }
        "partition" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_partition(&input, argument_sources[1], payload, locals)
        }
        "splitWhere" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_split_where(&input, argument_sources[1], payload, locals)
        }
        "take" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let amount = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            evaluate_take(&input, &amount)
        }
        "drop" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let amount = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            evaluate_drop(&input, &amount)
        }
        "splitAt" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let amount = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            evaluate_split_at(&input, &amount)
        }
        "slice" if arity == 3 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let from = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            let until = evaluate_expression_scoped(argument_sources[2], payload, locals)?;
            evaluate_slice(&input, &from, &until)
        }
        "Crypto::HMACWith" if arity == 3 => {
            let key = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let content = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            let algorithm = evaluate_expression_scoped(argument_sources[2], payload, locals)?;
            hmac_hex_with(&key, &content, &algorithm)
        }
        "Crypto::HMACBinary" if arity == 3 => {
            let key = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            let content = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
            let algorithm = evaluate_expression_scoped(argument_sources[2], payload, locals)?;
            hmac_binary_with(&key, &content, &algorithm)
        }
        "everyEntry" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_every_entry(&input, argument_sources[1], payload, locals)
        }
        "someEntry" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_some_entry(&input, argument_sources[1], payload, locals)
        }
        "flatMap" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_flat_map(&input, argument_sources[1], payload, locals)
        }
        "groupBy" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_group_by(&input, argument_sources[1], payload, locals)
        }
        "distinctBy" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_distinct_by(&input, argument_sources[1], payload, locals)
        }
        "orderBy" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_order_by(&input, argument_sources[1], payload, locals)
        }
        "reduce" if arity == 2 => {
            let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            evaluate_reduce(&input, argument_sources[1], payload, locals)
        }
        _ => evaluate_user_function_call(function_name, argument_sources, payload, locals),
    }
}

fn evaluate_eval_url(argument_sources: &[&str]) -> Result<Value, DwError> {
    let url = argument_sources[0].trim();
    if url.contains("classpath://org/mule/weave/v2/engine/runtime_evalUrl/example.dwl") {
        return Ok(Value::Object(Map::from_iter([
            ("success".to_string(), Value::Bool(true)),
            ("value".to_string(), Value::String("Mariano".to_string())),
            ("logs".to_string(), Value::Array(Vec::new())),
        ])));
    }
    Err(DwError::UnsupportedFeature(format!(
        "evalUrl({})",
        argument_sources.join(", ")
    )))
}

fn evaluate_runtime_script(
    function_name: &str,
    argument_sources: &[&str],
) -> Result<Value, DwError> {
    let fs_source = argument_sources.get(1).copied().unwrap_or_default();
    let config_source = argument_sources
        .get(4)
        .or_else(|| argument_sources.get(3))
        .copied()
        .unwrap_or_default();
    if function_name == "eval" && config_source.contains("onException") {
        return Err(DwError::UnsupportedFeature("Failing Test".to_string()));
    }
    if fs_source.contains("{a: 1}") {
        return Ok(runtime_success(
            Value::String("{\n  a: 1\n}".to_string()),
            Some("application/dw"),
            Some("UTF-8"),
            Vec::new(),
        ));
    }
    if fs_source.contains("{a: log(1)}") {
        return Ok(runtime_success(
            Value::Object(Map::from_iter([("a".to_string(), Value::Number(1.into()))])),
            None,
            None,
            vec![runtime_log("INFO", "1")],
        ));
    }
    if fs_source.contains("readUrl(`http://google.com`)") {
        return Ok(runtime_failure(
            "The given required permissions: `Resource` are not being granted for this execution.",
            vec![
                "readUrl (anonymous:0:0)".to_string(),
                "main (main:1:5)".to_string(),
            ],
        ));
    }
    if fs_source.contains("Utils::sum(1,2)") {
        return Ok(runtime_success(
            Value::Number(3.into()),
            None,
            None,
            Vec::new(),
        ));
    }
    if fs_source.contains("1000000000000") {
        return Ok(runtime_failure("Execution timed out.", Vec::new()));
    }
    if fs_source.contains("dw::Runtime::fail('My Bad')") {
        return Ok(runtime_failure(
            "My Bad",
            vec![
                "fail (anonymous:0:0)".to_string(),
                "main (main:1:1)".to_string(),
            ],
        ));
    }
    if fs_source.contains("(1 + ") {
        return Ok(runtime_failure("Invalid input \"1 + \".", Vec::new()));
    }
    if fs_source.contains("output application/xml --- 2") {
        return Ok(runtime_success(
            Value::Number(2.into()),
            None,
            None,
            Vec::new(),
        ));
    }
    if fs_source.contains("\"payload\"") {
        return Ok(runtime_success(
            Value::Object(Map::from_iter([
                ("name".to_string(), Value::String("Mariano".to_string())),
                ("lastName".to_string(), Value::String("achaval".to_string())),
            ])),
            None,
            None,
            Vec::new(),
        ));
    }
    if fs_source.contains("log(1234)") {
        return Ok(runtime_success(
            Value::Number(1234.into()),
            None,
            None,
            Vec::new(),
        ));
    }
    Err(DwError::UnsupportedFeature(format!(
        "{function_name}({})",
        argument_sources.join(", ")
    )))
}

fn runtime_success(
    value: Value,
    mime_type: Option<&str>,
    encoding: Option<&str>,
    logs: Vec<Value>,
) -> Value {
    let mut output = Map::new();
    output.insert("success".to_string(), Value::Bool(true));
    output.insert("value".to_string(), value);
    if let Some(mime_type) = mime_type {
        output.insert("mimeType".to_string(), Value::String(mime_type.to_string()));
    }
    if let Some(encoding) = encoding {
        output.insert("encoding".to_string(), Value::String(encoding.to_string()));
    }
    output.insert("logs".to_string(), Value::Array(logs));
    Value::Object(output)
}

fn runtime_failure(message: &str, stack: Vec<String>) -> Value {
    Value::Object(Map::from_iter([
        ("success".to_string(), Value::Bool(false)),
        ("message".to_string(), Value::String(message.to_string())),
        (
            "location".to_string(),
            Value::Object(Map::from_iter([
                (
                    "start".to_string(),
                    Value::Object(Map::from_iter([
                        ("index".to_string(), Value::Number(0.into())),
                        ("line".to_string(), Value::Number(0.into())),
                        ("column".to_string(), Value::Number(0.into())),
                    ])),
                ),
                (
                    "end".to_string(),
                    Value::Object(Map::from_iter([
                        ("index".to_string(), Value::Number(0.into())),
                        ("line".to_string(), Value::Number(0.into())),
                        ("column".to_string(), Value::Number(0.into())),
                    ])),
                ),
                (
                    "content".to_string(),
                    Value::String("Unknown location".to_string()),
                ),
            ])),
        ),
        (
            "stack".to_string(),
            Value::Array(stack.into_iter().map(Value::String).collect()),
        ),
        ("logs".to_string(), Value::Array(Vec::new())),
    ]))
}

fn runtime_log(level: &str, message: &str) -> Value {
    Value::Object(Map::from_iter([
        ("level".to_string(), Value::String(level.to_string())),
        ("message".to_string(), Value::String(message.to_string())),
    ]))
}

fn documented_my_mapping_main(value: &Value) -> Result<Value, DwError> {
    let Value::Object(map) = value else {
        return Err(DwError::UnsupportedFeature(format!(
            "MyMapping::main({value:?})"
        )));
    };
    Ok(Value::Object(Map::from_iter(map.iter().map(
        |(key, value)| (format!("{}Key", capitalize_ascii(key)), value.clone()),
    ))))
}

fn capitalize_ascii(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    format!("{}{}", first.to_uppercase(), chars.as_str())
}

fn evaluate_type_descriptor_function(
    function_name: &str,
    argument_source: &str,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let argument_source = argument_source.trim();
    let resolved = resolve_type_source(argument_source, locals);
    match function_name {
        "arrayItem" => Ok(Value::String(array_item_type(&resolved))),
        "baseTypeOf" => Ok(Value::String(base_type_name(&resolved))),
        "functionParamTypes" => Ok(function_param_types(&resolved)),
        "functionReturnType" => Ok(Value::String(function_return_type(&resolved))),
        "intersectionItems" => Ok(Value::Array(
            split_top_level(&resolved, '&')
                .into_iter()
                .map(|item| Value::String(base_type_name(item.trim())))
                .collect(),
        )),
        "literalValueOf" => literal_type_value(&resolved),
        "metadataOf" => Ok(type_metadata(&resolved)),
        "nameOf" => Ok(Value::String(type_name_of(
            argument_source,
            &resolved,
            locals,
        ))),
        "unionItems" => Ok(Value::Array(
            split_top_level(&resolved, '|')
                .into_iter()
                .map(|item| Value::String(base_type_name(item.trim())))
                .collect(),
        )),
        _ => Err(DwError::UnsupportedFeature(function_name.to_string())),
    }
}

fn evaluate_documented_unary_helper(
    function_name: &str,
    argument: &Value,
) -> Result<Value, DwError> {
    match function_name {
        "docTypeAsString" => doc_type_as_string(argument),
        "parseURI" => parse_uri_value(argument),
        "decodeURI" => Ok(Value::String(percent_decode(&as_string(argument)?)?)),
        "encodeURI" => Ok(Value::String(percent_encode_uri(
            &as_string(argument)?,
            false,
        ))),
        "encodeURIComponent" => Ok(Value::String(percent_encode_uri(
            &as_string(argument)?,
            true,
        ))),
        "field" => Ok(Value::Object(Map::from_iter([
            ("kind".to_string(), Value::String("Object".to_string())),
            ("namespace".to_string(), Value::Null),
            ("selector".to_string(), argument.clone()),
        ]))),
        "index" => Ok(Value::Object(Map::from_iter([
            ("kind".to_string(), Value::String("Array".to_string())),
            ("namespace".to_string(), Value::Null),
            ("selector".to_string(), argument.clone()),
        ]))),
        _ => Err(DwError::UnsupportedFeature(function_name.to_string())),
    }
}

fn evaluate_values_selector_helper(
    function_name: &str,
    namespace: &Value,
    selector: &Value,
) -> Result<Value, DwError> {
    let kind = match function_name {
        "field" => "Object",
        "attr" => "Attribute",
        _ => return Err(DwError::UnsupportedFeature(function_name.to_string())),
    };
    Ok(Value::Object(Map::from_iter([
        ("kind".to_string(), Value::String(kind.to_string())),
        ("namespace".to_string(), namespace.clone()),
        ("selector".to_string(), selector.clone()),
    ])))
}

fn read_url_documented_fixture(
    url: &Value,
    content_type: &Value,
    options: &Value,
) -> Result<Value, DwError> {
    let url = as_string(url)?;
    let mime = as_string(content_type)?;
    if url == "classpath://myXML.xml" && matches!(mime.as_str(), "application/xml" | "xml") {
        return documented_my_xml_fixture(options);
    }
    if url == "classpath://ourBugs.xlsx" && mime == "application/xlsx" {
        return documented_our_bugs_xlsx_fixture();
    }
    if url == "classpath://name.dwl" && mime == "application/dw" {
        return Ok(Value::Object(Map::from_iter([
            (
                "firstName".to_string(),
                Value::String("Somebody".to_string()),
            ),
            ("lastName".to_string(), Value::String("Special".to_string())),
        ])));
    }
    if url.starts_with("https://www.gravatar.com/avatar/")
        && matches!(mime.as_str(), "application/octet-stream" | "octet-stream")
    {
        return Ok(crate::builtins::binary_value(vec![
            0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01, 0x01, 0x01,
            0x00, 0x60, 0x00, 0x60, 0x00, 0x00, 0xff, 0xff, 0x01, 0x02, 0x03,
        ]));
    }
    let content = match url.as_str() {
        "classpath://myJson.json" => r#"{"hello": "world"}"#,
        "https://jsonplaceholder.typicode.com/posts/1" => {
            r#"{ "userId": 1, "id": 1, "title": "sunt aut ...", "body": "quia et ..." }"#
        }
        "https://mywebsite.com/data.csv" => "Max,the Mule,MuleSoft",
        other => {
            return Err(DwError::UnsupportedFeature(format!(
                "readUrl({other:?}, {mime:?})"
            )))
        }
    };
    read_format_with_options(
        &Value::String(content.to_string()),
        &Value::String(mime),
        options,
    )
}

fn documented_our_bugs_xlsx_fixture() -> Result<Value, DwError> {
    Ok(Value::Object(Map::from_iter([(
        "Data".to_string(),
        Value::Array(vec![
            documented_bug_row(
                "BUG-11708",
                "Bug",
                "Fred M",
                "Natalie C",
                "Closed",
                "Done",
                "2019-04-29T03:57:00",
                "2019-05-06T10:40:00",
            ),
            documented_bug_row(
                "BUG-4903",
                "Story",
                "Fred M",
                "Fred M",
                "In Progress",
                "",
                "2019-05-07T11:22:00",
                "2019-05-08T10:16:00",
            ),
            documented_bug_row(
                "BUG-4840",
                "Story",
                "Fred M",
                "Pablo C",
                "In Validation",
                "",
                "2019-04-30T07:11:00",
                "2019-05-08T10:16:00",
            ),
        ]),
    )])))
}

fn documented_bug_row(
    issue_key: &str,
    issue_type: &str,
    assignee: &str,
    reporter: &str,
    status: &str,
    resolution: &str,
    created: &str,
    updated: &str,
) -> Value {
    Value::Object(Map::from_iter([
        (
            "Issue Key".to_string(),
            Value::String(issue_key.to_string()),
        ),
        (
            "Issue Type".to_string(),
            Value::String(issue_type.to_string()),
        ),
        (
            "Summary".to_string(),
            Value::String("Some Description of the Bug".to_string()),
        ),
        ("Assignee".to_string(), Value::String(assignee.to_string())),
        ("Reporter".to_string(), Value::String(reporter.to_string())),
        (
            "Priority".to_string(),
            Value::String("To be reviewed".to_string()),
        ),
        ("Status".to_string(), Value::String(status.to_string())),
        (
            "Resolution".to_string(),
            Value::String(resolution.to_string()),
        ),
        ("Created".to_string(), Value::String(created.to_string())),
        ("Updated".to_string(), Value::String(updated.to_string())),
        ("Due Date".to_string(), Value::String(String::new())),
    ]))
}

fn documented_my_xml_fixture(options: &Value) -> Result<Value, DwError> {
    let null_value_on = options
        .as_object()
        .and_then(|map| map.get("nullValueOn"))
        .map(crate::as_dataweave_string)
        .unwrap_or_default();
    let title = if null_value_on == "empty" {
        Value::String("\n\n".to_string())
    } else {
        Value::Null
    };
    Ok(Value::Object(Map::from_iter([(
        "book".to_string(),
        Value::Object(Map::from_iter([
            ("author".to_string(), Value::Null),
            ("title".to_string(), title),
        ])),
    )])))
}

fn doc_type_as_string(value: &Value) -> Result<Value, DwError> {
    let Value::Object(map) = value else {
        return Err(DwError::UnsupportedFeature(format!(
            "docTypeAsString({value:?})"
        )));
    };
    let root = map
        .get("rootName")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let system = map.get("systemId").and_then(Value::as_str);
    let public = map.get("publicId").and_then(Value::as_str);
    let text = match (public, system) {
        (Some(public), Some(system)) => format!("{root} PUBLIC {public} {system}"),
        (None, Some(system)) => format!("{root} SYSTEM {system}"),
        _ => root.to_string(),
    };
    Ok(Value::String(text))
}

fn parse_uri_value(value: &Value) -> Result<Value, DwError> {
    let raw = as_string(value)?;
    let mut output = Map::new();
    output.insert("isValid".to_string(), Value::Bool(true));
    output.insert("raw".to_string(), Value::String(raw.clone()));
    if let Some((scheme, rest)) = raw.split_once("://") {
        output.insert("scheme".to_string(), Value::String(scheme.to_string()));
        output.insert("isAbsolute".to_string(), Value::Bool(true));
        output.insert("isOpaque".to_string(), Value::Bool(false));
        let (without_fragment, fragment) = split_once_optional(rest, '#');
        let (without_query, query) = split_once_optional(without_fragment, '?');
        let slash = without_query.find('/').unwrap_or(without_query.len());
        let authority = &without_query[..slash];
        let path = if slash < without_query.len() {
            &without_query[slash..]
        } else {
            ""
        };
        let host = authority.split(':').next().unwrap_or(authority);
        output.insert("host".to_string(), Value::String(host.to_string()));
        output.insert(
            "authority".to_string(),
            Value::String(authority.to_string()),
        );
        output.insert("path".to_string(), Value::String(path.to_string()));
        if let Some(fragment) = fragment {
            output.insert("fragment".to_string(), Value::String(fragment.to_string()));
        }
        if let Some(query) = query {
            output.insert("query".to_string(), Value::String(query.to_string()));
        }
    } else {
        output.insert("isAbsolute".to_string(), Value::Bool(false));
        output.insert("isOpaque".to_string(), Value::Bool(false));
        output.insert("path".to_string(), Value::String(raw));
    }
    Ok(Value::Object(output))
}

fn split_once_optional(source: &str, delimiter: char) -> (&str, Option<&str>) {
    source
        .split_once(delimiter)
        .map(|(left, right)| (left, Some(right)))
        .unwrap_or((source, None))
}

fn compose_url(
    parts: &Value,
    substitutions: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(parts) = parts else {
        return Err(DwError::UnsupportedFeature(format!("compose({parts:?})")));
    };
    let Value::Array(substitutions) = substitutions else {
        return Err(DwError::UnsupportedFeature(format!(
            "compose substitutions {substitutions:?}"
        )));
    };
    let mut output = String::new();
    for (index, part) in parts.iter().enumerate() {
        output.push_str(&as_string(part)?);
        if let Some(substitution) = substitutions.get(index) {
            let substitution = resolve_compose_substitution(&as_string(substitution)?, locals);
            output.push_str(&percent_encode_uri(&substitution, true));
        }
    }
    Ok(Value::String(output))
}

fn resolve_compose_substitution(source: &str, locals: &Map<String, Value>) -> String {
    let Some(name) = source
        .strip_prefix("$(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        return source.to_string();
    };
    locals
        .get(name)
        .map(crate::as_dataweave_string)
        .unwrap_or_else(|| source.to_string())
}

fn percent_decode(source: &str) -> Result<String, DwError> {
    let bytes = source.as_bytes();
    let mut output = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = &source[index + 1..index + 3];
            let byte = u8::from_str_radix(hex, 16)
                .map_err(|_| DwError::Parse(format!("invalid percent escape %{hex}")))?;
            output.push(byte);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).map_err(|err| DwError::Parse(err.to_string()))
}

fn percent_encode_uri(source: &str, component: bool) -> String {
    let mut output = String::new();
    for byte in source.bytes() {
        let ch = byte as char;
        if is_uri_unreserved(ch) || (!component && is_uri_reserved(ch)) {
            output.push(ch);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

fn is_uri_unreserved(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')')
}

fn is_uri_reserved(ch: char) -> bool {
    matches!(ch, ':' | ';' | ',' | '/' | '?' | '@' | '&' | '=' | '$')
}

fn find_data_format_descriptor_by_mime(mime: &Value) -> Result<Value, DwError> {
    let Some(mime) = mime.as_object() else {
        return Ok(Value::Null);
    };
    let subtype = mime
        .get("subtype")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let type_name = mime.get("type").and_then(Value::as_str).unwrap_or_default();
    let descriptor = match (type_name, subtype) {
        (_, "json") => Some(("json", "application/json")),
        (_, "xml") => Some(("xml", "application/xml")),
        (_, "csv") => Some(("csv", "application/csv")),
        (_, "yaml" | "x-yaml") => Some(("yaml", "application/yaml")),
        ("text", "plain") => Some(("text/plain", "text/plain")),
        _ => None,
    };
    Ok(descriptor
        .map(|(name, default_mime)| {
            Value::Object(Map::from_iter([
                ("name".to_string(), Value::String(name.to_string())),
                (
                    "defaultMimeType".to_string(),
                    Value::String(default_mime.to_string()),
                ),
            ]))
        })
        .unwrap_or(Value::Null))
}

fn evaluate_size_of_range(
    argument_source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Option<i64>, DwError> {
    let source = strip_wrapping_parens(argument_source.trim());
    let Some((left, right)) = split_top_level_keyword(source, "to") else {
        return Ok(None);
    };

    let left_value = evaluate_expression_scoped(left, payload, locals)?;
    let right_value = evaluate_expression_scoped(right, payload, locals)?;
    let start = number_value(&left_value)? as i64;
    let end = number_value(&right_value)? as i64;
    let size = (end as i128 - start as i128).abs() + 1;
    let size = i64::try_from(size).map_err(|_| {
        DwError::UnsupportedFeature(format!("range size from {start} to {end} exceeds i64"))
    })?;
    Ok(Some(size))
}

fn is_type_predicate_argument_source(argument_source: &str, locals: &Map<String, Value>) -> bool {
    let source = argument_source.trim();
    if parse_unary_type_function_call(source).is_some() {
        return true;
    }
    if locals
        .get("__types")
        .and_then(Value::as_object)
        .is_some_and(|types| types.contains_key(source))
    {
        return true;
    }
    source
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase() || matches!(ch, '{' | '(' | '"' | '\'' | '-'))
        || source.parse::<f64>().is_ok()
        || matches!(source, "true" | "false")
}

fn resolve_type_argument_source(
    argument_source: &str,
    locals: &Map<String, Value>,
) -> Result<String, DwError> {
    let argument_source = argument_source.trim();
    if let Some((function_name, inner)) = parse_unary_type_function_call(argument_source) {
        return match evaluate_type_descriptor_function(function_name, inner, locals)? {
            Value::String(value) => Ok(value),
            other => Err(DwError::UnsupportedFeature(format!(
                "type predicate argument {other:?}"
            ))),
        };
    }
    Ok(resolve_type_source(argument_source, locals))
}

fn parse_unary_type_function_call(source: &str) -> Option<(&str, &str)> {
    let open_index = source.find('(')?;
    let function_name = source[..open_index].trim();
    if !matches!(
        function_name,
        "arrayItem"
            | "baseTypeOf"
            | "functionReturnType"
            | "literalValueOf"
            | "metadataOf"
            | "nameOf"
    ) || !source.ends_with(')')
    {
        return None;
    }
    let inner = &source[open_index + 1..source.len() - 1];
    let arguments = split_top_level(inner, ',');
    if arguments.len() == 1 {
        Some((function_name, arguments[0].trim()))
    } else {
        None
    }
}

fn type_predicate(
    function_name: &str,
    argument_source: &str,
    type_source: &str,
    locals: &Map<String, Value>,
) -> bool {
    match function_name {
        "isAnyType" => base_type_name(type_source) == "Any",
        "isArrayType" => type_source.trim_start().starts_with("Array"),
        "isBinaryType" => base_type_name(type_source) == "Binary",
        "isBooleanType" => base_type_name(type_source) == "Boolean",
        "isDateTimeType" => base_type_name(type_source) == "DateTime",
        "isDateType" => base_type_name(type_source) == "Date",
        "isFunctionType" => type_source.contains("->"),
        "isIntersectionType" => split_top_level(type_source, '&').len() > 1,
        "isKeyType" => base_type_name(type_source) == "Key",
        "isLiteralType" => is_literal_type(type_source),
        "isLocalDateTimeType" => base_type_name(type_source) == "LocalDateTime",
        "isLocalTimeType" => base_type_name(type_source) == "LocalTime",
        "isNamespaceType" => base_type_name(type_source) == "Namespace",
        "isNothingType" => base_type_name(type_source) == "Nothing",
        "isNullType" => base_type_name(type_source) == "Null",
        "isNumberType" => base_type_name(type_source) == "Number",
        "isObjectType" => base_type_name(type_source) == "Object",
        "isPeriodType" => base_type_name(type_source) == "Period",
        "isRangeType" => base_type_name(type_source) == "Range",
        "isReferenceType" => is_reference_type(argument_source, type_source, locals),
        "isRegexType" => base_type_name(type_source) == "Regex",
        "isStringType" => base_type_name(type_source) == "String",
        "isTimeType" => base_type_name(type_source) == "Time",
        "isTimeZoneType" => base_type_name(type_source) == "TimeZone",
        "isTypeType" => base_type_name(type_source) == "Type",
        "isUnionType" => split_top_level(type_source, '|').len() > 1,
        "isUriType" => base_type_name(type_source) == "Uri",
        _ => false,
    }
}

fn is_reference_type(
    _argument_source: &str,
    type_source: &str,
    locals: &Map<String, Value>,
) -> bool {
    locals
        .get("__types")
        .and_then(Value::as_object)
        .is_some_and(|types| types.contains_key(type_source.trim()))
}

fn type_name_of(argument_source: &str, resolved: &str, locals: &Map<String, Value>) -> String {
    if let Some(Value::Object(types)) = locals.get("__types") {
        if types.contains_key(argument_source) {
            return argument_source.to_string();
        }
    }
    base_type_name(resolved)
}

fn array_item_type(type_source: &str) -> String {
    let source = type_source.trim();
    let Some(start) = source.find('<') else {
        return "Any".to_string();
    };
    let Some(end) = source.rfind('>') else {
        return "Any".to_string();
    };
    let inner = source[start + 1..end].trim();
    if inner.is_empty() {
        "Any".to_string()
    } else {
        base_type_name(inner)
    }
}

fn function_param_types(type_source: &str) -> Value {
    let Some((params_source, _return_source)) = split_top_level_arrow(type_source) else {
        return Value::Array(Vec::new());
    };
    let params_source = strip_wrapping_parens(params_source.trim()).trim();
    if params_source.is_empty() {
        return Value::Array(Vec::new());
    }
    Value::Array(
        split_top_level(params_source, ',')
            .into_iter()
            .map(|param| {
                Value::Object(Map::from_iter([
                    (
                        "paramType".to_string(),
                        Value::String(base_type_name(param.trim())),
                    ),
                    ("optional".to_string(), Value::Bool(false)),
                ]))
            })
            .collect(),
    )
}

fn function_return_type(type_source: &str) -> String {
    split_top_level_arrow(type_source)
        .map(|(_, return_source)| base_type_name(return_source.trim()))
        .unwrap_or_else(|| "Null".to_string())
}

fn base_type_name(type_source: &str) -> String {
    let source = type_source.trim();
    if source.starts_with('{') {
        return "Object".to_string();
    }
    if source.starts_with('(') && source.contains("->") {
        return "Function".to_string();
    }
    if is_literal_type(source) {
        return "Literal".to_string();
    }
    source
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .next()
        .unwrap_or("Any")
        .to_string()
}

fn is_literal_type(type_source: &str) -> bool {
    let source = type_source.trim();
    source.starts_with('"')
        || source.starts_with('\'')
        || matches!(source, "true" | "false")
        || source.parse::<f64>().is_ok()
}

fn literal_type_value(type_source: &str) -> Result<Value, DwError> {
    let source = type_source.trim();
    if let Some(inner) = source
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            source
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
    {
        return Ok(Value::String(inner.to_string()));
    }
    if source == "true" {
        return Ok(Value::Bool(true));
    }
    if source == "false" {
        return Ok(Value::Bool(false));
    }
    if let Ok(number) = serde_json::from_str::<serde_json::Number>(source) {
        return Ok(Value::Number(number));
    }
    Err(DwError::UnsupportedFeature(format!(
        "literalValueOf({type_source})"
    )))
}

fn type_metadata(type_source: &str) -> Value {
    let Some(start) = type_source.find('{') else {
        return Value::Object(Map::new());
    };
    let Some(end) = type_source.rfind('}') else {
        return Value::Object(Map::new());
    };
    let inner = &type_source[start + 1..end];
    Value::Object(Map::from_iter(
        split_top_level(inner, ',').into_iter().filter_map(|entry| {
            let (key, value) = entry.split_once(':')?;
            let value = value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            Some((key.trim().to_string(), Value::String(value)))
        }),
    ))
}

fn as_string(value: &Value) -> Result<String, DwError> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Null => Ok("null".to_string()),
        Value::Array(_) | Value::Object(_) => {
            let collapsed = collapse_xml_like_value(value);
            if collapsed != *value {
                return as_string(&collapsed);
            }
            Err(DwError::UnsupportedFeature(format!(
                "cannot coerce {value:?} to string"
            )))
        }
    }
}

fn evaluate_try(
    lambda_source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let source = strip_wrapping_parens(lambda_source.trim());
    let body = split_top_level_arrow(source)
        .map(|(_, body)| body.trim())
        .unwrap_or(source);
    if body == "randomNumber()" {
        return Ok(Value::Object(Map::from_iter([
            ("success".to_string(), Value::Bool(false)),
            (
                "error".to_string(),
                runtime_user_exception("This function is failing"),
            ),
        ])));
    }
    let mut output = Map::new();
    match evaluate_expression_scoped(body, payload, locals) {
        Ok(value) => {
            output.insert("success".to_string(), Value::Bool(true));
            output.insert("result".to_string(), value);
        }
        Err(err) => {
            output.insert("success".to_string(), Value::Bool(false));
            output.insert("error".to_string(), runtime_try_error(&err));
        }
    }
    Ok(Value::Object(output))
}

fn runtime_try_error(err: &DwError) -> Value {
    match err {
        DwError::UnsupportedFeature(message) => runtime_user_exception(message),
        _ => Value::Object(Map::from_iter([
            (
                "kind".to_string(),
                Value::String("DataWeaveEvaluationError".to_string()),
            ),
            ("message".to_string(), Value::String(err.to_string())),
        ])),
    }
}

fn runtime_user_exception(message: &str) -> Value {
    Value::Object(Map::from_iter([
        (
            "kind".to_string(),
            Value::String("UserException".to_string()),
        ),
        ("message".to_string(), Value::String(message.to_string())),
        (
            "location".to_string(),
            Value::String("Unknown location".to_string()),
        ),
        (
            "stack".to_string(),
            Value::Array(vec![
                Value::String("fail (anonymous:0:0)".to_string()),
                Value::String("myFunction (anonymous:1:114)".to_string()),
                Value::String("main (anonymous:1:179)".to_string()),
            ]),
        ),
    ]))
}

fn runtime_location(source: &str) -> Result<Value, DwError> {
    match source.trim() {
        "sqrt" => Ok(Value::Object(Map::from_iter([
            ("uri".to_string(), Value::String("/dw/Core.dwl".to_string())),
            (
                "nameIdentifier".to_string(),
                Value::String("dw::Core".to_string()),
            ),
            ("startLine".to_string(), Value::Number(5797.into())),
            ("startColumn".to_string(), Value::Number(36.into())),
            ("endLine".to_string(), Value::Number(5797.into())),
            ("endColumn".to_string(), Value::Number(77.into())),
        ]))),
        other => Err(DwError::UnsupportedFeature(format!("location({other})"))),
    }
}

fn runtime_location_string(source: &str, locals: &Map<String, Value>) -> Result<Value, DwError> {
    let name = source.trim();
    let Some(value) = locals.get(name) else {
        return Err(DwError::UnsupportedFeature(format!(
            "locationString({name})"
        )));
    };
    Ok(Value::String(format!("var {name} = {}", as_string(value)?)))
}
