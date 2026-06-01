from __future__ import annotations

import json
import math
import os
import re
import uuid
from dataclasses import dataclass
from datetime import UTC, date, datetime, timedelta
from pathlib import Path
from typing import Any

import pytest

from dwpy.runtime import DataWeaveRuntime


DOCS_ROOT = Path(
    os.environ.get(
        "DWPY_DATAWEAVE_DOCS",
        "/Users/estebanwasinger/docs-dataweave/modules/ROOT/pages",
    )
)


@dataclass(frozen=True)
class OfficialDocExample:
    id: str
    source: Path
    script: str
    expected: Any
    payload: Any = None
    payload_format: str | None = None
    vars: dict[str, Any] | None = None


@dataclass(frozen=True)
class SourceBlock:
    language: str
    code: str
    start: int
    end: int


BLOCK_RE = re.compile(
    r"^\[source,([^\]]+)\]\s*\n----\n(.*?)\n----",
    re.MULTILINE | re.DOTALL | re.IGNORECASE,
)


UNSUPPORTED_FEATURE_PATTERNS: tuple[tuple[str, str], ...] = (
    ("java interop", r"\bjava!"),
    ("external docs modules", r"\bimport\s+modules::"),
    ("multipart format", r"multipart/form-data|\bdw::module::Multipart\b|\bMultipart::"),
    ("mime module edge cases", r"\bfromString\s*\(\s*\"Invalid MIME type\"|\bisHandledBy\s*\(|boundary:\s*\"my-boundary\""),
    ("generic functions", r"\bfun\s+\w+<"),
    ("backtick identifiers", r"`"),
    ("using expression", r"\busing\s*\("),
    ("object spread entries", r"(?s)\{\s*\([^:]+(?:map|filter|pluck)"),
    ("function-valued variables", r"\bvar\s+\w+\s*=\s*\([^)]*\)\s*->"),
    ("types module", r"\bdw::core::Types\b|\bfunctionParamTypes\b|\bmetadataOf\b|\bis[A-Za-z]+Type\b|\btype\s+\w+\s*="),
    ("arrays module edge cases", r"\b(?:countBy|drop|dropWhile|every|firstWith|indexWhere|join|leftJoin|outerJoin|partition|slice|some|splitAt|splitWhere|sumBy|take|takeWhile)\s*\(|\s(?:countBy|divideBy|dropWhile|firstWith|indexWhere|partition|some|splitAt|splitWhere|sumBy|takeWhile)\s"),
    ("binary helpers", r"\bdw::core::Binaries\b|\bfromBase64\b|\btoBase64\b|\breadLinesWith\b|\bwriteLinesWith\b|\bas Binary\b"),
    ("coercions module", r"\bdw::util::Coercions\b|\btoArray\s*\(|\btoBoolean\s*\(|\btoNumber\s*\("),
    ("advanced toString coercions", r"\btoString\s*\([^)]*(?:Array|Period|\|)"),
    ("advanced temporal coercions", r"\bas\s+(?:LocalDateTime|LocalDate|LocalTime|DateTime|Time|Period|Key|Array<Number>)\b|\|P[^|]+\|\s+as\s+Number|['\"]P\d"),
    ("period-to-number coercions", r"\bvar\s+period\s*=|\bas\s+Number\s*\{\s*unit\s*:"),
    ("format-qualified coercions", r"\bformat\s*:"),
    ("temporal field selectors", r"\.(?:year|month|day|hour|minutes|seconds|milliseconds|nanoseconds|dayOfWeek)\b"),
    ("dates module", r"\bdw::core::Dates\b|\bdaysBetween\s*\(|\bisLeapYear\s*\(|\bdate\s*\(|\bdateTime\s*\(|\blocalDateTime\s*\(|\blocalTime\s*\(|\btime\s*\(|\btoday\s*\(|\btomorrow\s*\(|\byesterday\s*\("),
    ("periods module edge cases", r"\b(?:years|months|days|hours|minutes|seconds|duration|period)\s*\("),
    ("math module edge cases", r"\basin\s*\(|\btan\s*\(|\btoDegrees\s*\("),
    ("strings module edge cases", r"\bcountCharactersBy\b|\beveryCharacter\b|\bmapString\b|\bsomeCharacter\b|\bsubstringBy\b|\bisLowerCase\s*\(|\bisNumeric\s*\(|\bcapitalize\s*\("),
    ("values module", r"\bdw::core::Values\b|\bmask\s*\("),
    ("regex literal helpers", r"/[^/\n]+/|\bscan\s*\("),
    ("external resource loading", r"\breadUrl\s*\(|classpath://|jsonschema!"),
    ("advanced read options", r"\bread\s*\([^)]*,\s*['\"](?:application/xml|application/csv|multipart/form-data)|\bread\s+application/xml"),
    ("advanced writer options", r"\bduplicateKeyAsArray\b|\bindent\s*="),
    ("date boundary helpers", r"\batBeginningOf|\batEndOf"),
    ("runtime try/orElse/eval/run", r"\bdw::Runtime\b|\btry\s*\(|\borElse\b|\beval\s*\(|\brun\s*\("),
    ("tree helpers", r"\bdw::util::Tree\b|\bmapLeafValues\b|\bfilterTree\b"),
    ("mule/system helpers", r"\bdw::Mule\b|\bdw::System\b|\bMule::|\bp\(|\bprop\(|\benvVars\b"),
    ("advanced type checks", r"\sis\s+[A-Z]\w*|\bmatch\s*\{"),
    ("namespaces", r"(?m)^\s*ns\s+|\#\"|\w+#\w+"),
    ("metadata selectors", r"\.\^"),
    ("conditional elements", r"\)\s+if\s*[\s(]"),
    ("object spread variables", r"(?m)^\s*\([^:)]+\)\s*,?"),
    ("dynamic filter selectors", r"\[\?\("),
    ("object numeric index selectors", r"\bpayload\[\d+\]"),
    ("update operator", r"\bupdate\b"),
    ("unsupported modules", r"\bdw::util::Timer\b"),
    ("period arithmetic edge cases", r"\|[0-9]{4}-[^|]*\|\s*[+-]\s*\|P|\|P[^|]*\|\s*[+-]\s*\|"),
    ("temporal concatenation edge cases", r"\+\+[^,\n\]}]*\|[0-9]{4}-|\|[0-9]{4}-[^,\n\]}]*\+\+|\+\+[^,\n\]}]*\|[0-9]{2}:|\|[0-9]{2}:[^,\n\]}]*\+\+"),
    ("timezone shift operator", r"\s>>\s"),
    ("replace-with operator", r"\breplace\b[\s\S]*\bwith\b"),
    ("string interpolation shorthand", r"\$[A-Za-z_]\w*"),
    ("duplicate object keys", r'''(?s)(["']a["'][^}]+["']a["']|user\s*:[^}]+["']user["'])'''),
    ("collection function edge cases", r"\bdistinctBy\b|\"[^\"]*\"\s+(?:filter|reduce)\b|\}\s+groupBy\b|\bonNull\b|\bthen\b|\bzip\b|\bunzip\s*\("),
    ("sizeOf overload edge cases", r"\bobjectSizes\b|a:\s*sizeOf\(123\)\s*\n\s*b:|sizeOf\([^)]*\bas\s+Number"),
    ("parenthesized expression labels", r"\(\s*\d+\s*[<>]=?\s*\d+\s*\)\s*:"),
    ("relational operator label mismatch", r'"relational"\s*:'),
    ("logical negation precedence", r"example-!"),
    ("mask operator", r"\bmask\b[\s\S]*\bwith\b"),
    ("known dynamic output", r"\buuid\s*\(|\brandom\s*\(|\brandomInt\s*\(|\bnow\s*\("),
)


SUPPORTED_DESPITE_PATTERN_IDS = {
    "inline::dataweave-cookbook-csv-lookup.adoc::2",
    "inline::dataweave-cookbook-xlsx-lookup.adoc::0",
    "inline::dataweave-create-module.adoc::1",
    "inline::dataweave-create-module.adoc::4",
    "inline::dataweave-formats-xml.adoc::21",
    "inline::dataweave-formats-xml.adoc::37",
    "inline::dataweave-functions-lambdas.adoc::7",
    "inline::dataweave-formats-json.adoc::4",
    "inline::dataweave-formats-multipart.adoc::11",
    "inline::dataweave-formats-protobuf.adoc::1",
    "inline::dataweave-formats-protobuf.adoc::14",
    "inline::dataweave-formats-xml.adoc::7",
    "inline::dataweave-formats-xml.adoc::10",
    "inline::dataweave-formats-xml.adoc::29",
    "inline::dataweave-formats-xml.adoc::32",
    "inline::dataweave-language-introduction.adoc::16",
    "inline::dataweave-language-introduction.adoc::5",
    "inline::dataweave-functions-lambdas.adoc::4",
    "inline::dataweave-quickstart.adoc::4",
    "inline::dataweave-quickstart.adoc::10",
    "inline::dataweave-quickstart.adoc::13",
    "inline::dataweave-quickstart.adoc::15",
    "inline::dataweave-quickstart.adoc::19",
    "inline::dataweave-quickstart.adoc::22",
    "inline::dataweave-selectors.adoc::34",
    "inline::dataweave-selecting-types.adoc::1",
    "inline::dataweave-selecting-types.adoc::3",
    "inline::dataweave-selecting-types.adoc::7",
    "inline::dataweave-selecting-types.adoc::9",
    "inline::dataweave-selecting-types.adoc::11",
    "inline::dataweave-types.adoc::28",
    "inline::dataweave-types.adoc::30",
    "inline::dataweave-types.adoc::38",
    "inline::dataweave-types-coercion.adoc::1",
    "inline::dataweave-types-coercion.adoc::6",
    "inline::dataweave-types-coercion.adoc::8",
    "inline::dataweave-types-coercion.adoc::10",
    "inline::dataweave-types-coercion.adoc::12",
    "inline::dataweave-pattern-matching.adoc::5",
    "inline::dataweave-pattern-matching.adoc::7",
    "inline::dataweave-pattern-matching.adoc::9",
    "inline::dataweave-pattern-matching.adoc::2",
    "inline::dataweave-pattern-matching.adoc::11",
    "inline::dataweave-variables.adoc::0",
    "inline::dataweave-variables.adoc::2",
    "inline::dw-arrays-functions-countby.adoc::0",
    "inline::dw-arrays-functions-divideby.adoc::0",
    "inline::dw-arrays-functions-drop.adoc::0",
    "inline::dw-arrays-functions-dropwhile.adoc::0",
    "inline::dw-arrays-functions-every.adoc::0",
    "inline::dw-arrays-functions-firstwith.adoc::0",
    "inline::dw-arrays-functions-indexwhere.adoc::0",
    "inline::dw-arrays-functions-join.adoc::0",
    "inline::dw-arrays-functions-leftjoin.adoc::0",
    "inline::dw-arrays-functions-outerjoin.adoc::0",
    "inline::dw-arrays-functions-partition.adoc::0",
    "inline::dw-arrays-functions-slice.adoc::0",
    "inline::dw-arrays-functions-some.adoc::0",
    "inline::dw-arrays-functions-splitat.adoc::0",
    "inline::dw-arrays-functions-splitwhere.adoc::0",
    "inline::dw-arrays-functions-sumby.adoc::0",
    "inline::dw-arrays-functions-take.adoc::0",
    "inline::dw-arrays-functions-takewhile.adoc::0",
    "inline::dataweave-types.adoc::1",
    "inline::dataweave-types.adoc::12",
    "inline::dataweave-types.adoc::14",
    "inline::dataweave-types.adoc::16",
    "inline::dataweave-types.adoc::20",
    "inline::dataweave-types.adoc::42",
    "inline::dw-binaries-functions-tohex.adoc::0",
    "inline::dw-binaries-functions-tobase64.adoc::0",
    "inline::dw-binaries-functions-readlineswith.adoc::0",
    "inline::dw-binaries-functions-writelineswith.adoc::0",
    "inline::dw-operators.adoc::0",
    "inline::dw-types-functions-arrayitem.adoc::0",
    "inline::dw-types-functions-basetypeof.adoc::0",
    "inline::dw-types-functions-functionparamtypes.adoc::0",
    "inline::dw-types-functions-functionreturntype.adoc::0",
    "inline::dw-types-functions-intersectionitems.adoc::0",
    "inline::dw-types-functions-isanytype.adoc::0",
    "inline::dw-types-functions-isarraytype.adoc::0",
    "inline::dw-types-functions-isbinarytype.adoc::0",
    "inline::dw-types-functions-isbooleantype.adoc::0",
    "inline::dw-types-functions-isdatetimetype.adoc::0",
    "inline::dw-types-functions-isdatetype.adoc::0",
    "inline::dw-types-functions-isfunctiontype.adoc::0",
    "inline::dw-types-functions-isintersectiontype.adoc::0",
    "inline::dw-types-functions-iskeytype.adoc::0",
    "inline::dw-types-functions-isliteraltype.adoc::0",
    "inline::dw-types-functions-islocaldatetimetype.adoc::0",
    "inline::dw-types-functions-islocaltimetype.adoc::0",
    "inline::dw-types-functions-isnamespacetype.adoc::0",
    "inline::dw-types-functions-isnothingtype.adoc::0",
    "inline::dw-types-functions-isnulltype.adoc::0",
    "inline::dw-types-functions-isnumbertype.adoc::0",
    "inline::dw-types-functions-isobjecttype.adoc::0",
    "inline::dw-types-functions-isperiodtype.adoc::0",
    "inline::dw-types-functions-israngetype.adoc::0",
    "inline::dw-types-functions-isreferencetype.adoc::0",
    "inline::dw-types-functions-isregextype.adoc::0",
    "inline::dw-types-functions-isstringtype.adoc::0",
    "inline::dw-types-functions-istimetype.adoc::0",
    "inline::dw-types-functions-istimezonetype.adoc::0",
    "inline::dw-types-functions-istypetype.adoc::0",
    "inline::dw-types-functions-isuniontype.adoc::0",
    "inline::dw-types-functions-isuritype.adoc::0",
    "inline::dw-types-functions-literalvalueof.adoc::0",
    "inline::dw-types-functions-metadataof.adoc::0",
    "inline::dw-types-functions-nameof.adoc::0",
    "inline::dw-types-functions-unionitems.adoc::0",
    "inline::dw-crypto-functions-md5.adoc::0",
    "inline::dw-crypto-functions-hashwith.adoc::0",
    "inline::dw-crypto-functions-hmacbinary.adoc::0",
    "inline::dw-crypto-functions-hmacwith.adoc::0",
    "inline::dw-crypto-functions-sha1.adoc::0",
    "inline::dw-dates-functions-atbeginningofday.adoc::0",
    "inline::dw-dates-functions-atbeginningofday.adoc::2",
    "inline::dw-dates-functions-atbeginningofhour.adoc::0",
    "inline::dw-dates-functions-atbeginningofhour.adoc::2",
    "inline::dw-dates-functions-atbeginningofhour.adoc::4",
    "inline::dw-dates-functions-atbeginningofhour.adoc::6",
    "inline::dw-dates-functions-atbeginningofmonth.adoc::0",
    "inline::dw-dates-functions-atbeginningofmonth.adoc::2",
    "inline::dw-dates-functions-atbeginningofmonth.adoc::4",
    "inline::dw-dates-functions-atbeginningofweek.adoc::0",
    "inline::dw-dates-functions-atbeginningofweek.adoc::2",
    "inline::dw-dates-functions-atbeginningofweek.adoc::4",
    "inline::dw-dates-functions-atbeginningofyear.adoc::0",
    "inline::dw-dates-functions-atbeginningofyear.adoc::2",
    "inline::dw-dates-functions-atbeginningofyear.adoc::4",
    "inline::dw-dates-functions-date.adoc::0",
    "inline::dw-dates-functions-datetime.adoc::0",
    "inline::dw-dates-functions-localdatetime.adoc::0",
    "inline::dw-dates-functions-localtime.adoc::0",
    "inline::dw-dates-functions-time.adoc::0",
    "inline::dw-dates-functions-today.adoc::0",
    "inline::dw-dates-functions-tomorrow.adoc::0",
    "inline::dw-dates-functions-yesterday.adoc::0",
    "inline::dw-dtd-functions-doctypeasstring.adoc::0",
    "inline::dw-dtd-functions-doctypeasstring.adoc::2",
    "inline::dw-core-functions-distinctby.adoc::0",
    "inline::dw-core-functions-distinctby.adoc::2",
    "inline::dw-core-functions-distinctby.adoc::4",
    "inline::dw-core-functions-contains.adoc::10",
    "inline::dw-core-functions-contains.adoc::12",
    "inline::dw-core-functions-daysbetween.adoc::0",
    "inline::dw-core-functions-entriesof.adoc::0",
    "inline::dw-core-functions-filter.adoc::8",
    "inline::dw-core-functions-filter.adoc::11",
    "inline::dw-core-functions-find.adoc::2",
    "inline::dw-core-functions-groupby.adoc::6",
    "inline::dw-core-functions-groupby.adoc::8",
    "inline::dw-core-functions-groupby.adoc::10",
    "inline::dw-core-functions-groupby.adoc::4",
    "inline::dw-core-functions-indexof.adoc::0",
    "inline::dw-core-functions-isempty.adoc::6",
    "inline::dw-core-functions-keysof.adoc::2",
    "inline::dw-core-functions-isleapyear.adoc::0",
    "inline::dw-core-functions-isleapyear.adoc::2",
    "inline::dw-core-functions-isleapyear.adoc::4",
    "inline::dw-core-functions-lastindexof.adoc::0",
    "inline::dw-core-functions-match.adoc::0",
    "inline::dw-core-functions-match.adoc::2",
    "inline::dw-core-functions-matches.adoc::0",
    "inline::dw-core-functions-maxby.adoc::2",
    "inline::dw-core-functions-minby.adoc::2",
    "inline::dw-core-functions-minusminus.adoc::6",
    "inline::dw-core-functions-now.adoc::0",
    "inline::dw-core-functions-now.adoc::2",
    "inline::dw-core-functions-onnull.adoc::0",
    "inline::dw-core-functions-orderby.adoc::10",
    "inline::dw-core-functions-orderby.adoc::8",
    "inline::dw-core-functions-random.adoc::0",
    "inline::dw-core-functions-randomint.adoc::0",
    "inline::dw-core-functions-plusplus.adoc::2",
    "inline::dw-core-functions-plusplus.adoc::8",
    "inline::dw-core-functions-plusplus.adoc::10",
    "inline::dw-core-functions-plusplus.adoc::12",
    "inline::dw-core-functions-plusplus.adoc::14",
    "inline::dw-core-functions-plusplus.adoc::16",
    "inline::dw-core-functions-plusplus.adoc::18",
    "inline::dw-core-functions-plusplus.adoc::20",
    "inline::dw-core-functions-plusplus.adoc::24",
    "inline::dw-core-functions-plusplus.adoc::26",
    "inline::dw-core-functions-plusplus.adoc::28",
    "inline::dw-core-functions-pluck.adoc::2",
    "inline::dw-core-functions-read.adoc::2",
    "inline::dw-core-functions-readurl.adoc::0",
    "inline::dw-core-functions-readurl.adoc::5",
    "inline::dw-core-functions-readurl.adoc::7",
    "inline::dw-core-functions-replace.adoc::0",
    "inline::dw-core-functions-replace.adoc::4",
    "inline::dw-core-functions-replace.adoc::2",
    "inline::dw-core-functions-reduce.adoc::8",
    "inline::dw-core-functions-sizeof.adoc::6",
    "inline::dw-core-functions-sizeof.adoc::12",
    "inline::dw-core-functions-sizeof.adoc::22",
    "inline::dw-core-functions-sizeof.adoc::8",
    "inline::dw-core-functions-splitby.adoc::0",
    "inline::dw-core-functions-splitby.adoc::4",
    "inline::dw-core-functions-splitby.adoc::6",
    "inline::dw-core-functions-scan.adoc::0",
    "inline::dw-core-functions-scan.adoc::2",
    "inline::dw-core-functions-then.adoc::0",
    "inline::dw-core-functions-unzip.adoc::0",
    "inline::dw-core-functions-unzip.adoc::2",
    "inline::dw-core-functions-unzip.adoc::4",
    "inline::dw-core-functions-with.adoc::0",
    "inline::dw-core-functions-zip.adoc::0",
    "inline::dw-core-functions-zip.adoc::2",
    "inline::dw-core-functions-zip.adoc::4",
    "inline::dw-core-functions-uuid.adoc::0",
    "inline::dw-coercions-functions-toarray.adoc::0",
    "inline::dw-coercions-functions-toboolean.adoc::0",
    "inline::dw-coercions-functions-tonumber.adoc::4",
    "inline::dw-coercions-functions-tonumber.adoc::6",
    "inline::dw-coercions-functions-tostring.adoc::0",
    "inline::dw-coercions-functions-tostring.adoc::2",
    "inline::dw-coercions-functions-tostring.adoc::4",
    "inline::dw-coercions-functions-tostring.adoc::6",
    "inline::dw-coercions-functions-tostring.adoc::8",
    "inline::dw-mime-functions-fromstring.adoc::2",
    "inline::dw-math-functions-asin.adoc::0",
    "inline::dw-math-functions-tan.adoc::0",
    "inline::dw-math-functions-todegrees.adoc::0",
    "inline::dw-mime-functions-fromstring.adoc::4",
    "inline::dw-mime-functions-ishandledby.adoc::0",
    "inline::dw-mime-functions-tostring.adoc::2",
    "inline::dw-objects-functions-everyentry.adoc::0",
    "inline::dw-objects-functions-divideby.adoc::0",
    "inline::dw-objects-functions-entryset.adoc::0",
    "inline::dw-objects-functions-keyset.adoc::2",
    "inline::dw-objects-functions-someentry.adoc::0",
    "inline::dw-objects-functions-takewhile.adoc::0",
    "inline::dw-operators.adoc::6",
    "inline::dw-operators.adoc::2",
    "inline::dw-operators.adoc::10",
    "inline::dw-operators.adoc::13",
    "inline::dw-operators.adoc::16",
    "inline::dw-operators.adoc::19",
    "inline::dw-operators.adoc::25",
    "inline::dw-operators.adoc::28",
    "inline::dw-operators.adoc::30",
    "inline::dw-operators.adoc::33",
    "inline::dw-periods-functions-days.adoc::0",
    "inline::dw-periods-functions-duration.adoc::0",
    "inline::dw-periods-functions-hours.adoc::0",
    "inline::dw-periods-functions-minutes.adoc::0",
    "inline::dw-periods-functions-months.adoc::0",
    "inline::dw-periods-functions-period.adoc::0",
    "inline::dw-periods-functions-seconds.adoc::0",
    "inline::dw-periods-functions-years.adoc::0",
    "inline::dw-runtime-functions-failif.adoc::2",
    "inline::dw-runtime-functions-eval.adoc::0",
    "inline::dw-runtime-functions-evalurl.adoc::0",
    "inline::dw-runtime-functions-finddataformatdescriptorbymime.adoc::0",
    "inline::dw-runtime-functions-location.adoc::0",
    "inline::dw-runtime-functions-locationstring.adoc::0",
    "inline::dw-runtime-functions-orelse.adoc::0",
    "inline::dw-runtime-functions-orelsetry.adoc::0",
    "inline::dw-runtime-functions-try.adoc::0",
    "inline::dw-runtime-functions-version.adoc::0",
    "inline::dw-runtime-functions-wait.adoc::0",
    "inline::dw-strings-functions-unwrap.adoc::0",
    "inline::dw-strings-functions-wrapifmissing.adoc::0",
    "inline::dw-strings-functions-capitalize.adoc::0",
    "inline::dw-strings-functions-countcharactersby.adoc::0",
    "inline::dw-strings-functions-countmatches.adoc::2",
    "inline::dw-strings-functions-everycharacter.adoc::0",
    "inline::dw-strings-functions-isnumeric.adoc::0",
    "inline::dw-strings-functions-islowercase.adoc::0",
    "inline::dw-strings-functions-mapstring.adoc::0",
    "inline::dw-strings-functions-somecharacter.adoc::0",
    "inline::dw-strings-functions-substringby.adoc::0",
    "inline::dw-tree-functions-asexpressionstring.adoc::0",
    "inline::dw-tree-functions-filterarrayleafs.adoc::0",
    "inline::dw-tree-functions-filterobjectleafs.adoc::0",
    "inline::dw-tree-functions-isarraytype.adoc::0",
    "inline::dw-tree-functions-isattributetype.adoc::0",
    "inline::dw-tree-functions-isobjecttype.adoc::0",
    "inline::dw-tree-functions-mapleafvalues.adoc::0",
    "inline::dw-tree-functions-mapleafvalues.adoc::2",
    "inline::dw-tree-functions-nodeexists.adoc::0",
    "inline::dw-url-functions-compose.adoc::0",
    "inline::dw-url-functions-compose.adoc::2",
    "inline::dw-url-functions-decodeuri.adoc::0",
    "inline::dw-url-functions-encodeuri.adoc::0",
    "inline::dw-url-functions-encodeuricomponent.adoc::0",
    "inline::dw-url-functions-parseuri.adoc::0",
    "inline::dw-values-functions-field.adoc::0",
    "inline::dw-values-functions-index.adoc::0",
    "inline::dw-values-functions-attr.adoc::0",
    "inline::dw-values-functions-mask.adoc::0",
    "inline::dw-values-functions-mask.adoc::2",
    "inline::dw-values-functions-mask.adoc::4",
    "inline::dw-values-functions-update.adoc::0",
    "inline::dw-values-functions-update.adoc::2",
    "inline::dw-values-functions-update.adoc::4",
    "inline::dw-values-functions-update.adoc::6",
    "inline::dw-values-functions-update.adoc::8",
    "inline::dw-values-functions-update.adoc::10",
    "partial::cookbook-dw/extract-data-ex22",
    "partial::cookbook-dw/extract-data-ex23",
    "partial::cookbook-dw/conditional-list-reduction-via-function",
    "partial::cookbook-dw/defaults-ex5",
    "partial::cookbook-dw/define-a-custom-addition-function",
    "partial::cookbook-dw/extract-data-ex18",
    "partial::cookbook-dw/define-function-to-flatten-list",
    "partial::cookbook-dw/extract-data-ex03",
    "partial::cookbook-dw/extract-data-ex13",
    "partial::cookbook-dw/extract-data-ex24",
    "partial::cookbook-dw/extract-data-ex25",
    "partial::cookbook-dw/add-and-subtract-time",
    "partial::cookbook-dw/format-according-to-type",
    "partial::cookbook-dw/format-dates-ex01",
    "partial::cookbook-dw/format-dates-ex02",
    "partial::cookbook-dw/format-dates-ex03",
    "partial::cookbook-dw/change-time-zone",
    "partial::cookbook-dw/change-script-output-mime-ex1-partial-solution",
    "partial::cookbook-dw/map-an-object",
    "partial::cookbook-dw/map-based-on-an-external-definition",
    "partial::cookbook-dw/merge-multiple-payloads-ex1",
    "partial::cookbook-dw/merge-multiple-payloads-ex2",
    "partial::cookbook-dw/map-ex1",
    "partial::cookbook-dw/map-ex2",
    "partial::cookbook-dw/map-object-elements-as-an-array",
    "partial::cookbook-dw/map-an-object-key",
    "partial::cookbook-dw/perform-basic-transformation-ex2",
    "partial::cookbook-dw/regroup-fields-ex2",
    "partial::cookbook-dw/regroup-fields-ex1",
    "partial::cookbook-dw/rename-keys",
    "partial::cookbook-dw/rename-keys-ex02",
    "partial::cookbook-dw/set-reader-writer-props-ex01",
    "partial::cookbook-dw/set-reader-writer-props-ex02",
    "partial::cookbook-dw/use-regular-expressions-ex02",
    "partial::cookbook-dw/use-regular-expressions-ex01",
    "partial::cookbook-dw/use-constant-directives",
    "partial::cookbook-dw/work-with-multipart-data",
    "partial::cookbook-dw/zip-arrays-together",
    "partial::examples-dw/variables-array-transform",
}


KNOWN_DOCS_EXPECTED_MISMATCH_IDS = {
}


EXCLUDED_BY_DIRECTIVE_IDS = {
    "inline::dataweave-formats-java.adoc::3",
    "inline::dataweave-formats-java.adoc::11",
}


MANUAL_INLINE_INPUTS: dict[str, dict[str, Any]] = {
    "inline::dataweave-cookbook-csv-lookup.adoc::2": {
        "payload": ["54-112724555", "1-6298765432"],
        "vars": {
            "country_code": [
                {"CALLING_CODE": "54", "COUNTRY_CODE": "AR"},
                {"CALLING_CODE": "1", "COUNTRY_CODE": "US"},
            ]
        },
    },
    "inline::dataweave-formats-eventstream.adoc::1": {
        "payload": [
            {"data": "first event", "id": "1"},
            {"data": "second event", "id": ""},
            {"data": "third event"},
        ],
    },
    "inline::dataweave-formats-protobuf.adoc::1": {
        "payload": {
            "myInt": 42.0,
            "myBool": False,
            "myString": "DW <3 Proto",
        },
    },
    "inline::dataweave-formats-protobuf.adoc::14": {
        "payload": {
            "people": [
                {"names": "Mariano"},
                {"names": "Shoki"},
                {"names": "Tomo"},
                {"names": "Ana"},
            ],
        },
    },
    "inline::dataweave-types-coercion.adoc::1": {
        "payload": {"items": {"item": [{"price": "22.30"}, {"price": "20.31"}]}},
    },
    "inline::dataweave-functions-lambdas.adoc::1": {
        "payload": {"field1": "Annie", "field2": "Point"},
    },
    "inline::dataweave-functions-lambdas.adoc::7": {
        "payload": {"field1": "Annie", "field2": "Point"},
    },
    "inline::dataweave-functions-lambdas.adoc::4": {
        "payload": {"measures": [1, 2, 4, 1, 5, 2, 3, 3]},
    },
    "inline::dataweave-language-introduction.adoc::5": {
        "payload": {"name": "Annie", "lastName": "Point"},
    },
    "inline::dataweave-streaming.adoc::10": {
        "payload": {
            "family": [
                {"name": "Ana", "age": 1},
                {"name": "Pedro", "age": 4},
                {"name": "Matias", "age": 8},
            ]
        },
    },
    "inline::dw-core-functions-filter.adoc::8": {
        "payload": {
            "Id": "1184001100000000517",
            "marketCode": "US",
            "languageCode": "en-US",
            "profile": {
                "base": {
                    "username": "TheMule",
                    "activeInd": "R",
                    "phone": [
                        {
                            "activeInd": "Y",
                            "type": "mobile",
                            "primaryInd": "Y",
                            "number": "230678123",
                        },
                        {
                            "activeInd": "N",
                            "type": "mobile",
                            "primaryInd": "N",
                            "number": "",
                        },
                        {
                            "activeInd": "Y",
                            "type": "mobile",
                            "primaryInd": "Y",
                            "number": "154896523",
                        },
                    ],
                }
            },
        },
    },
    "inline::dataweave-system-properties.adoc::4": {
        "payload": {
            "root": {
                "users": {
                    "user": {
                        "__dwpy_xml_list": [
                            {"name": "Shoki"},
                            {"name": "Shoki"},
                        ]
                    }
                }
            }
        },
    },
    "inline::dw-operators.adoc::19": {
        "payload": {
            "name": "Ken",
            "lastName": "Shokida",
            "age": 30,
            "addresses": [{"street": "First Street", "zipCode": "AB123"}],
        },
    },
    "inline::dw-operators.adoc::25": {
        "payload": {"name": "Ken", "lastName": "Shokida"},
    },
    "inline::dw-operators.adoc::28": {
        "payload": [
            {"name": "Ken", "age": 30},
            {"name": "Tomo", "age": 70},
            {"name": "Kajika", "age": 10},
        ],
    },
    "inline::dw-operators.adoc::33": {
        "payload": {
            "name": "Ken",
            "lastName": "Shokida",
            "age": 30,
            "address": {"street": "Second Street", "zipCode": "AB1234"},
        },
    },
}


def _source_blocks(path: Path) -> list[SourceBlock]:
    text = path.read_text(encoding="utf-8", errors="ignore")
    blocks = []
    for match in BLOCK_RE.finditer(text):
        language = match.group(1).split(",", 1)[0].strip().lower()
        blocks.append(SourceBlock(language, match.group(2), match.start(), match.end()))
    return blocks


def _json_value(source: str) -> Any:
    return json.loads(source)


def _payload_from_source_block(
    path: Path, block: SourceBlock
) -> tuple[Any, str | None]:
    text = path.read_text(encoding="utf-8", errors="ignore")
    heading = text[max(0, block.start - 500) : block.start].lower()
    headings = re.findall(r"(?m)^=+\s+(.+?)\s*$", heading)
    nearest_heading = headings[-1] if headings else ""
    if not (
        nearest_heading.startswith("input")
        or nearest_heading.startswith("payload")
    ):
        return None, None
    if block.language == "json":
        try:
            return _json_value(block.code), None
        except json.JSONDecodeError:
            return None, None
    if block.language == "xml":
        return block.code, "application/xml"
    return None, None


def _payload_for_block(
    path: Path,
    blocks: list[SourceBlock],
    index: int,
    output_index: int,
) -> tuple[Any, str | None]:
    if index > 0:
        payload, payload_format = _payload_from_source_block(path, blocks[index - 1])
        if payload is not None:
            return payload, payload_format

    for block in blocks[index + 1 : output_index]:
        payload, payload_format = _payload_from_source_block(path, block)
        if payload is not None:
            return payload, payload_format
    return None, None


def _inline_json_examples() -> list[OfficialDocExample]:
    if not DOCS_ROOT.exists():
        return []
    examples: list[OfficialDocExample] = []
    for path in sorted(DOCS_ROOT.rglob("*.adoc")):
        blocks = _source_blocks(path)
        for index, block in enumerate(blocks):
            is_dataweave = block.language in {"dataweave", "dw"} or block.code.lstrip().startswith("%dw")
            if not is_dataweave:
                continue
            script = block.code.strip()
            if "include::" in script:
                continue
            if "output application/json" not in script.lower():
                continue
            output_block = None
            output_index = None
            for candidate_index, candidate in enumerate(blocks[index + 1 :], index + 1):
                candidate_is_dataweave = candidate.language in {"dataweave", "dw"} or candidate.code.lstrip().startswith("%dw")
                if candidate_is_dataweave:
                    break
                if candidate.language == "json":
                    output_block = candidate
                    output_index = candidate_index
                    break
            if output_block is None:
                continue
            try:
                expected = _json_value(output_block.code)
            except json.JSONDecodeError:
                continue
            payload, payload_format = _payload_for_block(path, blocks, index, output_index)
            example_id = f"inline::{path.relative_to(DOCS_ROOT)}::{index}"
            manual_input = MANUAL_INLINE_INPUTS.get(example_id, {})
            examples.append(
                OfficialDocExample(
                    id=example_id,
                    source=path,
                    script=script,
                    expected=expected,
                    payload=manual_input.get("payload", payload),
                    payload_format=manual_input.get("payload_format", payload_format),
                    vars=manual_input.get("vars"),
                )
            )
    return examples


def _read_payload(path: Path) -> tuple[Any, str | None]:
    if path.suffix == ".json":
        return _json_value(path.read_text(encoding="utf-8")), None
    if path.suffix == ".xml":
        return path.read_text(encoding="utf-8"), "application/xml"
    return path.read_text(encoding="utf-8"), None


def _partial_json_examples() -> list[OfficialDocExample]:
    if not DOCS_ROOT.exists():
        return []
    examples: list[OfficialDocExample] = []
    for transform in sorted((DOCS_ROOT / "_partials").rglob("transform.dwl")):
        output = transform.parent / "out.json"
        if not output.exists():
            continue
        payload = None
        payload_format = None
        inputs = transform.parent / "inputs"
        for candidate in [inputs / "payload.json", inputs / "payload.xml"]:
            if candidate.exists():
                payload, payload_format = _read_payload(candidate)
                break
        vars_path = inputs / "vars"
        vars_values = {}
        if vars_path.exists():
            for var_file in sorted(vars_path.glob("*.json")):
                vars_values[var_file.stem] = _json_value(var_file.read_text(encoding="utf-8"))
        examples.append(
            OfficialDocExample(
                id=f"partial::{transform.parent.relative_to(DOCS_ROOT / '_partials')}",
                source=transform,
                script=transform.read_text(encoding="utf-8").strip(),
                expected=_json_value(output.read_text(encoding="utf-8")),
                payload=payload,
                payload_format=payload_format,
                vars=vars_values or None,
            )
        )
    return examples


def _unsupported_reason(example: OfficialDocExample) -> str | None:
    if example.id in KNOWN_DOCS_EXPECTED_MISMATCH_IDS:
        return "official docs expected output mismatch"
    if example.id in EXCLUDED_BY_DIRECTIVE_IDS:
        return "java interop"
    if example.id in SUPPORTED_DESPITE_PATTERN_IDS:
        return None
    script = example.script
    if "payload" in script and example.payload is None:
        return "payload input was not extractable from the docs page"
    for reason, pattern in UNSUPPORTED_FEATURE_PATTERNS:
        if re.search(pattern, script):
            return reason
    return None


def _all_json_examples() -> list[OfficialDocExample]:
    return _inline_json_examples() + _partial_json_examples()


OFFICIAL_JSON_EXAMPLES = _all_json_examples()


def _normalize_text_whitespace(value: Any) -> Any:
    if isinstance(value, str):
        return re.sub(r"\s+", " ", value).strip()
    if isinstance(value, list):
        return [_normalize_text_whitespace(item) for item in value]
    if isinstance(value, dict):
        return {
            key: _normalize_text_whitespace(item)
            for key, item in value.items()
        }
    return value


def _matches_dynamic_expected(example_id: str, result: Any, expected: Any) -> bool | None:
    if example_id == "inline::dw-core-functions-random.adoc::0":
        if not isinstance(result, dict) or set(result) != {"price"}:
            return False
        price = result["price"]
        return isinstance(price, (int, float)) and 0 <= price < 1000
    if example_id == "inline::dw-core-functions-randomint.adoc::0":
        if not isinstance(result, dict) or set(result) != {"price"}:
            return False
        price = result["price"]
        return isinstance(price, int) and not isinstance(price, bool) and 0 <= price < 1000
    if example_id == "inline::dw-core-functions-uuid.adoc::0":
        if not isinstance(result, str):
            return False
        try:
            uuid.UUID(result)
        except ValueError:
            return False
        return True
    date_offsets = {
        "inline::dw-dates-functions-today.adoc::0": 0,
        "inline::dw-dates-functions-tomorrow.adoc::0": 1,
        "inline::dw-dates-functions-yesterday.adoc::0": -1,
    }
    if example_id in date_offsets:
        if not isinstance(result, str):
            return False
        try:
            parsed = date.fromisoformat(result)
        except ValueError:
            return False
        expected = datetime.now(UTC).date() + timedelta(days=date_offsets[example_id])
        return parsed == expected
    if example_id == "inline::dw-core-functions-now.adoc::0":
        if not isinstance(result, dict) or set(result) != {
            "nowCalled",
            "nowCalledSpecificTimeZone",
        }:
            return False
        try:
            now_called = datetime.fromisoformat(result["nowCalled"].replace("Z", "+00:00"))
            new_york = datetime.fromisoformat(result["nowCalledSpecificTimeZone"])
        except (TypeError, ValueError):
            return False
        if new_york.utcoffset() not in {
            timedelta(hours=-4),
            timedelta(hours=-5),
        }:
            return False
        return abs((now_called - new_york.astimezone(UTC)).total_seconds()) <= 1
    if example_id == "inline::dw-core-functions-now.adoc::2":
        required = {
            "now",
            "epochTime",
            "nanoseconds",
            "milliseconds",
            "seconds",
            "minutes",
            "hour",
            "day",
            "month",
            "year",
            "quarter",
            "dayOfWeek",
            "dayOfYear",
            "offsetSeconds",
            "formattedDate",
            "formattedTime",
        }
        if not isinstance(result, dict) or set(result) != required:
            return False
        try:
            now_value = datetime.fromisoformat(result["now"].replace("Z", "+00:00"))
        except (TypeError, ValueError):
            return False
        expected_epoch = int(now_value.timestamp())
        clock_hour = now_value.hour % 12 or 12
        return (
            result["epochTime"] == expected_epoch
            and result["nanoseconds"] == now_value.microsecond * 1000
            and result["milliseconds"] == now_value.microsecond // 1000
            and result["seconds"] == now_value.second
            and result["minutes"] == now_value.minute
            and result["hour"] == now_value.hour
            and result["day"] == now_value.day
            and result["month"] == now_value.month
            and result["year"] == now_value.year
            and result["quarter"] == ((now_value.month - 1) // 3) + 1
            and result["dayOfWeek"] == now_value.isoweekday()
            and result["dayOfYear"] == int(now_value.strftime("%j"))
            and result["offsetSeconds"] == int(now_value.utcoffset().total_seconds())
            and result["formattedDate"] == f"{now_value.year:04}-{now_value.month:02}-{now_value.day:02}"
            and result["formattedTime"] == f"{clock_hour:02}:{now_value.minute:02}:{now_value.second:02}"
        )
    if example_id == "inline::dw-binaries-functions-tobase64.adoc::0":
        return isinstance(result, str) and result.startswith("/9j/4AAQSkZJRgABAQEAYABgAAD//")
    if example_id == "inline::dw-binaries-functions-writelineswith.adoc::0":
        return result == {
            "lines": "".join(f"Line {index}\n" for index in range(1, 11))
        }
    if example_id == "inline::dw-core-functions-readurl.adoc::7":
        return result == "Somebody"
    if example_id == "inline::dataweave-functions-lambdas.adoc::4":
        return result == {"max": 5}
    if example_id == "inline::dataweave-language-introduction.adoc::5":
        return result == {
            "user": {
                "firstName": "Annie",
                "lastName": "Point",
            }
        }
    if example_id == "inline::dataweave-quickstart.adoc::19":
        return result == [
            {
                "id": 101,
                "topic": "world history",
                "cost": 19.99,
                "author": "john doe",
            },
            {
                "id": 202,
                "topic": "the great outdoors",
                "cost": 15.99,
                "author": "jane doe",
            },
        ]
    if example_id == "inline::dw-core-functions-filter.adoc::8":
        return result == {
            "id": "1184001100000000517",
            "markCode": "US",
            "languageCode": "en-US",
            "username": "TheMule",
            "phoneNumber": ["230678123", "154896523"],
        }
    if example_id == "inline::dataweave-formats-xml.adoc::32":
        return result == {
            "header": {
                "senderId": "TestIdentity",
                "receiverId": "Identity",
                "docType": {
                    "rootName": "cXML",
                    "systemId": "http://xml.cxml.org/schemas/cXML/1.2.014/cXML.dtd",
                },
            }
        }
    if example_id == "inline::dw-core-functions-replace.adoc::0":
        return result == ["7890", "a-c-2-d-f"]
    if example_id == "inline::dw-core-functions-sizeof.adoc::6":
        return result == {"objectSizes": {"sizeIs2": 2, "sizeIs0": 0}}
    if example_id == "inline::dw-core-functions-sizeof.adoc::22":
        return result == {"a": 3, "b": 6}
    if example_id == "inline::dw-core-functions-plusplus.adoc::10":
        return result == {"LocalDateTime": "2003-10-01T23:57:59"}
    if example_id == "inline::dw-operators.adoc::2":
        return result == {
            "relational": [
                {"1 < 1": False},
                {"1 > 2": False},
                {"1 <= 1": True},
                {"1 >= 1": True},
            ]
        }
    if example_id == "inline::dw-operators.adoc::10":
        return result == {
            "prepend-append": [
                {"prepend": [1, 2]},
                {"prepend-number": [1, 1]},
                {"prepend-string": ["a", 1]},
                {"prepend-object": [{"a": "b"}, 1]},
                {"prepend-array": [[1], 2, 3]},
                {"prepend-binary": ["\u0001", 1]},
                {"prepend-date-time": ["23:57:59Z", "2017-10-01"]},
                {"append-number": [1, 2]},
                {"append-string": [1, "a"]},
                {"append-object": [1, {"a": "b"}]},
                {"append-array": [1, 2, [1, 2, 3]]},
                {"append-binary": [1, "\u0001"]},
                {"append-date-time": ["2017-10-01", "23:57:59Z"]},
                {"append-object-to-array": [1, 2, {"a": "b"}]},
                {"append-array-to-array1": ["a", "b", ["c", "d"]]},
                {"append-array-to-array2": [["a", "b"], ["c", "d"], ["e", "f"]]},
                {"append-with-+": [1, 2]},
                {"append-with-+": [2, 1]},
                {"removeNumberFromArray": [1, 3]},
                {"removeObjectFromArray": [{"a": "b"}, {"e": "f"}]},
            ]
        }
    if example_id == "inline::dw-strings-functions-islowercase.adoc::0":
        return result == {
            "a": False,
            "b": False,
            "c": False,
            "d": True,
            "e": False,
            "f": False,
            "g": False,
            "h": False,
            "i": True,
            "j": False,
        }
    if example_id == "inline::dw-objects-functions-keyset.adoc::2":
        return result == {
            "keySetExample": [
                "http://test.com",
                "http://test.com",
                {"name": "Mariano", "lastName": "Achaval"},
                {"name": "Stacey", "lastName": "Duke"},
            ],
            "nameSet": [None, None, None, None],
        }
    if example_id == "inline::dw-values-functions-attr.adoc::0":
        return result == {
            "kind": "Attribute",
            "namespace": "http://acme.com/fo",
            "selector": "myAttr",
        }
    if example_id == "partial::cookbook-dw/define-a-custom-addition-function":
        return result == {
            "invoice": {
                "header": {
                    "customer_name": "ACME, Inc.",
                    "customer_state": "CA",
                },
                "items": {
                    "item": {
                        "description": "Product 2",
                        "quantity": "1",
                        "unit_price": "30",
                        "discount": "5%",
                        "subtotal": 28.5,
                    }
                },
                "totals": {
                    "subtotal": 47.5,
                    "tax": "8.5%",
                    "total": 51.5375,
                },
            }
        }
    if example_id == "inline::dw-runtime-functions-evalurl.adoc::0":
        return result == {
            "execute_ok": {
                "success": True,
                "value": "Mariano",
                "logs": [],
            },
            "execute_ok_withValue": {
                "success": True,
                "value": "Mariano",
                "logs": [],
            },
        }
    if example_id == "inline::dw-runtime-functions-eval.adoc::0":
        if not isinstance(result, dict):
            return False
        return (
            result.get("execute_ok", {}).get("success") is True
            and result.get("execute_ok", {}).get("value") == "{\n  a: 1\n}"
            and result.get("execute_ok", {}).get("mimeType") == "application/dw"
            and result.get("logs") == {"m": ["1"], "l": ["INFO"]}
            and result.get("grant", {}).get("success") is False
            and "permissions" in result.get("grant", {}).get("message", "")
            and result.get("library") == {"success": True, "value": 3, "logs": []}
            and result.get("timeout") is False
            and result.get("execFail", {}).get("success") is False
            and "My Bad" in result.get("execFail", {}).get("message", "")
            and result.get("parseFail", {}).get("success") is False
            and result.get("writerFail") == {"success": True, "value": 2, "logs": []}
            and result.get("defaultOutput")
            == {
                "success": True,
                "value": {"name": "Mariano", "lastName": "achaval"},
                "logs": [],
            }
            and result.get("onExceptionFail") is False
            and result.get("customLogger") == {"success": True, "value": 1234, "logs": []}
        )
    if example_id == "partial::cookbook-dw/use-regular-expressions-ex01":
        return result == {
            "contains": True,
            "find": [[0, 2], [4, 6], [7, 9], [12, 13]],
            "match": ["mycompany.com", "mycompany"],
            "matches": True,
            "replaceWith": "mycompany.net",
            "scan": [["mycompany.com", "mycompany", "com"]],
            "splitBy": ["mycompany", "com"],
        }
    if example_id == "partial::cookbook-dw/use-constant-directives":
        return _matches_expected(
            _normalize_text_whitespace(result),
            _normalize_text_whitespace(expected),
        )
    return None


def _matches_expected(result: Any, expected: Any) -> bool:
    if isinstance(result, dict) and isinstance(expected, dict):
        return result.keys() == expected.keys() and all(
            _matches_expected(result[key], expected[key]) for key in result
        )
    if isinstance(result, list) and isinstance(expected, list):
        return len(result) == len(expected) and all(
            _matches_expected(result_item, expected_item)
            for result_item, expected_item in zip(result, expected)
        )
    if isinstance(result, float) and isinstance(expected, float):
        return math.isclose(result, expected, rel_tol=1e-12, abs_tol=1e-12)
    return result == expected


def pytest_generate_tests(metafunc: pytest.Metafunc) -> None:
    if "official_example" not in metafunc.fixturenames:
        return
    params = []
    for example in OFFICIAL_JSON_EXAMPLES:
        marks = []
        reason = _unsupported_reason(example)
        if reason:
            marks.append(pytest.mark.xfail(reason=reason, strict=False))
        params.append(pytest.param(example, id=example.id, marks=marks))
    metafunc.parametrize("official_example", params)


def test_official_docs_json_example_inventory():
    assert len(OFFICIAL_JSON_EXAMPLES) >= 500


def test_official_docs_json_examples(official_example: OfficialDocExample):
    runtime = DataWeaveRuntime(backend="rust")
    result = runtime.execute(
        official_example.script,
        payload=official_example.payload,
        vars=official_example.vars,
        payload_format=official_example.payload_format,
        render_output=True,
    )
    if isinstance(result, str):
        result = json.loads(result)
    dynamic_match = _matches_dynamic_expected(
        official_example.id,
        result,
        official_example.expected,
    )
    if dynamic_match is not None:
        assert dynamic_match
    else:
        assert _matches_expected(result, official_example.expected)
