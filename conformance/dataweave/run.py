"""Compare strict Rust execution with pinned dataweave-native execution.

Case files contain // case NAME, // expect RESULT and full DataWeave scripts.
Live runs verify stored expectations with the reference; --offline uses stored results.
"""

import argparse
import json
import sys
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass
from decimal import Decimal
from importlib.metadata import version
from pathlib import Path
from typing import Protocol

HERE = Path(__file__).resolve().parent
REFERENCE_PACKAGE = "dataweave-native"
REFERENCE_VERSION = "2.12.2"
ERROR = "error"
REFERENCE_FAILURE = "reference-error"


class Obj(list[tuple[str, object]]):
    """A JSON object as its key/value pairs: DataWeave objects can repeat keys."""


class RustRuntime(Protocol):
    def execute(self, script_source: str, payload: object) -> object: ...

    def last_execution_engine(self) -> str: ...


@dataclass
class Case:
    file: Path
    name: str
    expect: str
    script: str
    line: int  # of `// expect`, 0-based


def parse_cases(file: Path) -> list[Case]:
    lines = file.read_text(encoding="utf-8").splitlines()
    starts = [i for i, line in enumerate(lines) if line.startswith("// case ")]
    if not starts:
        raise ValueError(f"{file.name}: no cases")
    cases = []
    for start, end in zip(starts, [*starts[1:], len(lines)], strict=True):
        body = lines[start + 1 : end]
        if not body or not body[0].startswith("// expect "):
            raise ValueError(
                f"{file.name}:{start + 1}: `// case` needs an `// expect` line"
            )
        expect = body.pop(0).removeprefix("// expect ").strip()
        while body and not body[-1].strip():
            body.pop()
        script = "\n".join(body)
        if not expect or not lines[start].removeprefix("// case ").strip():
            raise ValueError(f"{file.name}:{start + 1}: empty case name or expectation")
        if not script.lstrip().startswith("%dw"):
            raise ValueError(
                f"{file.name}:{start + 1}: case requires a full script beginning with `%dw`"
            )
        cases.append(
            Case(
                file,
                lines[start].removeprefix("// case ").strip(),
                expect,
                script,
                start + 1,
            )
        )
    names = [case.name for case in cases]
    if len(set(names)) != len(names):
        raise ValueError(f"{file.name}: duplicate case names")
    return cases


def compact(value: object) -> str:
    if isinstance(value, Obj):
        return (
            "{"
            + ",".join(
                f"{json.dumps(key, ensure_ascii=False)}:{compact(item)}"
                for key, item in value
            )
            + "}"
        )
    if isinstance(value, list):
        return "[" + ",".join(compact(item) for item in value) + "]"
    if isinstance(value, Decimal):
        return str(value)
    return json.dumps(value, ensure_ascii=False)


def load(text: str) -> object:
    return json.loads(text, object_pairs_hook=Obj, parse_float=Decimal)


def same_value(left: object, right: object) -> bool:
    """Ignore JSON number spelling and whitespace; retain kinds, order and duplicate keys."""
    if isinstance(left, Obj) or isinstance(right, Obj):
        return (
            isinstance(left, Obj)
            and isinstance(right, Obj)
            and len(left) == len(right)
            and all(a == b and same_value(x, y) for (a, x), (b, y) in zip(left, right))
        )
    if isinstance(left, list) or isinstance(right, list):
        return (
            isinstance(left, list)
            and isinstance(right, list)
            and len(left) == len(right)
            and all(same_value(a, b) for a, b in zip(left, right))
        )
    if isinstance(left, bool) or isinstance(right, bool):
        return type(left) is type(right) and left == right
    return left == right


def same_output(left: str, right: str) -> bool:
    try:
        return same_value(load(left), load(right))
    except ValueError:
        return left == right


def new_rust_runtime() -> RustRuntime:
    from dwpy._dwpy_rust import RustDataWeaveRuntime

    runtime = RustDataWeaveRuntime(allow_legacy_fallback=False)
    if "rust-core-evaluator" not in runtime.capabilities():
        raise ValueError("Compiled Rust bridge required; run maturin develop first")
    return runtime


def check(case: Case, result: str, runtime: RustRuntime) -> str | None:
    """Compare outputs or script rejection; the reference binding does not expose error phases."""
    from dwpy.parser import ParseError
    from dwpy.runtime import DataWeaveEvaluationError

    try:
        output = runtime.execute(case.script, None)
    except NotImplementedError as exc:
        return f"unsupported in Rust: {exc}"
    except ParseError as exc:
        return None if result == ERROR else f"Rust syntax error: {exc}"
    except DataWeaveEvaluationError as exc:
        # The bridge also wraps some feature gaps in this evaluation exception.
        if str(exc).startswith(
            "Rust evaluator does not yet support this DataWeave feature"
        ):
            return f"unsupported in Rust: {exc}"
        return None if result == ERROR else f"Rust evaluation error: {exc}"
    except Exception as exc:  # noqa: BLE001 - report unexpected engine crashes as mismatches
        return f"Rust crash ({type(exc).__name__}): {exc}"
    if runtime.last_execution_engine() != "rust-core":
        return "execution did not use the Rust core"
    if result == ERROR:
        return f"Rust accepts a script rejected by the reference ({result}); output: {output!r}"
    if not isinstance(output, str):
        return f"Rust did not return rendered text: {output!r}"
    try:
        encoded = _output(
            output, None if result.startswith("text ") else "application/json"
        )
    except ValueError as exc:
        return f"Rust returned invalid JSON: {exc}; output: {output!r}"
    return (
        None
        if same_output(encoded, result)
        else f"reference: {result!r}; Rust: {output!r}"
    )


def _output(text: str, mime_type: str | None) -> str:
    """Normalize declared JSON only; retain every character of other output formats."""
    media_type = (mime_type or "").partition(";")[0].strip().lower()
    if media_type == "application/json" or media_type.endswith("+json"):
        return compact(load(text))
    return "text " + json.dumps(text, ensure_ascii=False)


class NativeResult(Protocol):
    success: bool
    error: str | None
    mime_type: str | None

    def get_string(self) -> str | None: ...


class NativeRuntime(Protocol):
    def run(self, script: str) -> NativeResult: ...


@contextmanager
def new_reference_runtime() -> Iterator[NativeRuntime]:
    installed = version(REFERENCE_PACKAGE)
    if installed != REFERENCE_VERSION:
        raise ValueError(
            f"Expected {REFERENCE_PACKAGE}=={REFERENCE_VERSION}, found {installed}"
        )
    import dataweave

    try:
        with dataweave.DataWeave() as runtime:
            yield runtime
    except dataweave.DataWeaveError as exc:
        raise ValueError(f"Reference engine setup/execution failed: {exc}") from exc


def run_engine(
    cases: list[Case], messages: dict[int, str] | None = None
) -> dict[int, str]:
    """Reference results by case index; binding diagnostics never assert script rejection."""
    results = {}
    with new_reference_runtime() as runtime:
        for index, case in enumerate(cases):
            result = runtime.run(case.script)
            if result.success:
                text = result.get_string()
                if text is None:
                    raise ValueError(f"Reference returned no output for {case.name}")
                results[index] = _output(text, result.mime_type)
            else:
                # These diagnostics come from the pinned binding's response decoder.
                # Missing diagnostics likewise cannot establish a script rejection.
                results[index] = (
                    REFERENCE_FAILURE
                    if not result.error
                    or result.error.startswith(
                        (
                            "Native returned null",
                            "Native returned empty response",
                            "Failed to parse native JSON response:",
                            "Native response JSON is not an object",
                        )
                    )
                    else ERROR
                )
                if messages is not None:
                    messages[index] = result.error or "Reference failed without a diagnostic"
    return results


def update(cases: list[Case], results: dict[int, str]) -> int:
    """Write the engine's results into `?` and stale expectations; returns how many changed."""
    if any("\n" in result or "\r" in result for result in results.values()):
        raise ValueError("Expectations must be encoded on one line")
    changed = 0
    for file in dict.fromkeys(case.file for case in cases):
        lines = file.read_text(encoding="utf-8").splitlines()
        file_changed = False
        for index, case in enumerate(cases):
            if results[index] == REFERENCE_FAILURE:
                continue  # Keep verified expectations through reference outages.
            expected = results[index]
            if case.file == file and case.expect != expected:
                lines[case.line] = f"// expect {expected}"
                changed += 1
                file_changed = True
        if file_changed:
            file.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return changed


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="*", type=Path, help="case files (default: all)")
    parser.add_argument("--case", help="only the case with this name")
    parser.add_argument(
        "--offline",
        action="store_true",
        help="compare Rust with stored reference expectations",
    )
    parser.add_argument(
        "--update", action="store_true", help="record reference results as expectations"
    )
    parser.add_argument("--check", action="store_true", help="exit 1 on mismatches")
    parser.add_argument(
        "--report",
        type=Path,
        help="write per-case results and reproduction commands as JSON",
    )
    args = parser.parse_args(argv)
    if args.offline and args.update:
        parser.error("--update needs the reference engine")
    try:
        files = args.files or sorted((HERE / "cases").glob("*.dwl"))
        cases = [
            case
            for file in files
            for case in parse_cases(file)
            if args.case in (None, case.name)
        ]
        if not cases:
            raise ValueError("No cases selected")
        runtime = new_rust_runtime()
        messages: dict[int, str] = {}
        results = {} if args.offline else run_engine(cases, messages)
        if args.update:
            print(f"{update(cases, results)} expectation(s) updated from the reference")
            cases = [
                case
                for file in files
                for case in parse_cases(file)
                if args.case in (None, case.name)
            ]
        failures = reference_failures = 0
        records = []
        for index, case in enumerate(cases):
            where = f"{case.file.name}:{case.line + 1} {case.name}"
            reference = case.expect if args.offline else results[index]
            try:
                file_name = case.file.resolve().relative_to(HERE.parents[1]).as_posix()
            except ValueError:
                file_name = str(case.file)
            command = [
                "uv",
                "run",
                "--extra",
                "conformance",
                "python",
                "-m",
                "conformance.dataweave.run",
                file_name,
                "--case",
                case.name,
                "--check",
            ]
            if args.offline:
                command.append("--offline")
            record = {
                "id": f"{case.file.stem}/{case.name}",
                "file": file_name,
                "line": case.line + 1,
                "expected": case.expect,
                "reference": reference,
                "reference_error": messages.get(index),
                "status": "pass",
                "detail": None,
                "reproduce": command,
            }
            records.append(record)
            unavailable = reference == REFERENCE_FAILURE or (
                args.offline and reference == "?"
            )
            if unavailable:
                reference_failures += 1
                record["status"] = "reference-error"
                record["detail"] = (
                    messages.get(index)
                    or "No verified reference result; run live to retry"
                )
                print(f"REFERENCE {where}: {record['detail']}")
                continue
            if case.expect == "?" or not same_output(reference, case.expect):
                failures += 1
                record["status"] = "fail"
                record["detail"] = (
                    "Stored expectation differs from the reference; review --update"
                )
                print(
                    f"FAIL {where}: stored {case.expect!r}; reference {reference!r} (use --update to record)"
                )
                continue
            problem = check(case, reference, runtime)
            if problem is not None:
                failures += 1
                record["status"] = "fail"
                record["detail"] = problem
                print(f"FAIL {where}: {problem}")
        mode = (
            "stored reference expectations"
            if args.offline
            else f"{REFERENCE_PACKAGE} {REFERENCE_VERSION}"
        )
        print(
            f"{len(cases)} case(s), {failures} failure(s), "
            f"{reference_failures} reference failure(s) ({mode})"
        )
        if args.report:
            args.report.parent.mkdir(parents=True, exist_ok=True)
            report = {
                "reference": {
                    "package": REFERENCE_PACKAGE,
                    "version": REFERENCE_VERSION,
                    "live": not args.offline,
                },
                "summary": {
                    "total": len(cases),
                    "passed": len(cases) - failures - reference_failures,
                    "failed": failures,
                    "reference_failures": reference_failures,
                },
                "cases": records,
            }
            args.report.write_text(
                json.dumps(report, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
        return 1 if args.check and (failures or reference_failures) else 0
    except (ImportError, OSError, ValueError, RuntimeError) as exc:
        print(f"SETUP: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
