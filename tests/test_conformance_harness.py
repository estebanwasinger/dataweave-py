from contextlib import nullcontext
from importlib.metadata import PackageNotFoundError
from pathlib import Path
from types import SimpleNamespace

import pytest

from conformance.dataweave import run
from dwpy.parser import ParseError
from dwpy.runtime import DataWeaveEvaluationError

SCRIPT = "%dw 2.0\noutput application/json\n---\n1"


def case_text(
    name: str = "x",
    expect: str = "?",
    script: str = SCRIPT,
) -> str:
    return f"// case {name}\n// expect {expect}\n{script}\n"


def case_file(tmp_path: Path, text: str) -> Path:
    file = tmp_path / "cases.dwl"
    file.write_text(text, encoding="utf-8")
    return file


def test_case_headers_and_locations(tmp_path: Path) -> None:
    script = "%dw 2.0\noutput application/json\n---\n1 + 2"
    file = case_file(tmp_path, case_text("add", "3", script))
    case = run.parse_cases(file)[0]
    assert case.script == script
    assert (case.name, case.expect, case.line) == (
        "add",
        "3",
        1,
    )


@pytest.mark.parametrize(
    "text",
    [
        "",
        "// case x\n1",
        "// case x\n// expect \n1",
        "// case \n// expect 1\n1",
        case_text(expect="1") + case_text(expect="2"),
    ],
)
def test_malformed_cases_fail(tmp_path: Path, text: str) -> None:
    with pytest.raises(ValueError):
        run.parse_cases(case_file(tmp_path, text))


@pytest.mark.parametrize(
    "script",
    [
        '%dw 2.0\noutput application/json\n---\n"---"',
        '%dw 2.0\noutput application/json\n---\n"a\n---\nb"',
        '%dw 2.0\n/*\n---\noutput application/xml\n*/\noutput application/json\n---\n1',
        '%dw 2.0\noutput application/json\n---\ndo { var x = "---" --- x }',
        "%dw 2.0\n1",  # deliberately invalid full script must reach the engine intact
    ],
)
def test_full_scripts_are_preserved(tmp_path: Path, script: str) -> None:
    case = run.parse_cases(case_file(tmp_path, "// case full\n// expect 1\n" + script))[
        0
    ]
    assert case.script == script


def test_explicit_output_is_preserved(tmp_path: Path) -> None:
    script = "%dw 2.0\noutput application/xml\n---\n{a: 1}"
    case = run.parse_cases(case_file(tmp_path, "// case xml\n// expect ?\n" + script))[
        0
    ]
    assert case.script == script


@pytest.mark.parametrize("script", ['"a\n---\nb"', "1", "var x = 1\n---\nx"])
def test_fragments_require_explicit_scripts(tmp_path: Path, script: str) -> None:
    with pytest.raises(ValueError, match="full script"):
        run.parse_cases(case_file(tmp_path, case_text(script=script)))


@pytest.mark.parametrize(
    ("left", "right", "same"),
    [
        ('{"a": [1, 2.0]}', '{ "a": [1.0, 2] }', True),
        ("1e100", "1" + "0" * 100, True),
        ("0.12345678901234567890", "0.12345678901234567891", False),
        ("true", "1", False),
        ("false", "0", False),
        ('{"a":1,"a":2}', '{"a":2}', False),
        ('{"a":1,"a":2}', '{"a":1,"a":2}', True),
        ('{"a":1,"b":2}', '{"b":2,"a":1}', False),
        ("{}", "[]", False),
        ("[1,2]", "[2,1]", False),
        ("null", '"null"', False),
        ('"1"', "1", False),
        ("<a>1</a>\n", "<a>1</a>", False),
    ],
)
def test_comparison_retains_semantic_differences(
    left: str, right: str, same: bool
) -> None:
    assert run.same_output(left, right) is same


class FakeRust:
    def __init__(
        self,
        output: object = "1",
        error: Exception | None = None,
        engine: str = "rust-core",
    ) -> None:
        self.output = output
        self.error = error
        self.engine = engine

    def execute(self, script_source: str, payload: object) -> object:
        if self.error:
            raise self.error
        return self.output

    def last_execution_engine(self) -> str:
        return self.engine


def sample_case() -> run.Case:
    return run.Case(Path("sample.dwl"), "sample", "1", SCRIPT, 1)


@pytest.mark.parametrize(
    ("result", "runtime", "passes"),
    [
        ("1", FakeRust(), True),
        ("2", FakeRust(), False),
        ("1", FakeRust(engine="python-legacy"), False),
        ("1", FakeRust(output=1), False),
        (run.ERROR, FakeRust(), False),
        (run.ERROR, FakeRust(error=ParseError("bad syntax")), True),
        ("1", FakeRust(error=ParseError("bad syntax")), False),
        (run.ERROR, FakeRust(error=DataWeaveEvaluationError("error")), True),
        (run.ERROR, FakeRust(error=NotImplementedError("unsupported")), False),
        (
            run.ERROR,
            FakeRust(
                error=DataWeaveEvaluationError(
                    "Rust evaluator does not yet support this DataWeave feature: ???"
                )
            ),
            False,
        ),
        (run.ERROR, FakeRust(error=RuntimeError("crash")), False),
    ],
)
def test_rejections_and_fallback_cannot_silently_pass(
    result: str, runtime: FakeRust, passes: bool
) -> None:
    assert (run.check(sample_case(), result, runtime) is None) is passes


@pytest.mark.parametrize(
    ("output", "exit_code"),
    [
        ("1", 0),
        ("2", 1),
    ],
)
def test_offline_exit(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    output: str,
    exit_code: int,
) -> None:
    file = case_file(tmp_path, case_text("value", "1"))
    monkeypatch.setattr(run, "new_rust_runtime", lambda: FakeRust(output))

    def no_reference(*args: object) -> None:
        pytest.fail("offline mode invoked the reference")

    monkeypatch.setattr(run, "run_engine", no_reference)
    assert run.main([str(file), "--offline", "--check"]) == exit_code


def test_update_only_records_reference_results(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    file = case_file(tmp_path, case_text(script=SCRIPT.removesuffix("1") + "2"))
    monkeypatch.setattr(run, "new_rust_runtime", lambda: FakeRust("999"))
    monkeypatch.setattr(run, "run_engine", lambda *args: {0: "2"})
    assert run.main([str(file), "--update", "--check"]) == 1
    assert "// expect 2\n" in file.read_text()
    assert "999" not in file.read_text()


def test_multiline_update_does_not_modify_cases(tmp_path: Path) -> None:
    file = case_file(tmp_path, case_text())
    before = file.read_bytes()
    with pytest.raises(ValueError, match="one line"):
        run.update(run.parse_cases(file), {0: "a\nb"})
    assert file.read_bytes() == before


@pytest.mark.parametrize("compiled", [False, True])
def test_runtime_requires_compiled_bridge_and_disables_fallback(
    monkeypatch: pytest.MonkeyPatch, compiled: bool
) -> None:
    import dwpy._dwpy_rust as bridge

    def construct(*, allow_legacy_fallback: bool) -> SimpleNamespace:
        assert allow_legacy_fallback is False
        return SimpleNamespace(
            capabilities=lambda: (
                ["rust-core-evaluator"] if compiled else ["source-checkout-fallback"]
            )
        )

    monkeypatch.setenv("DWPY_TEST_BACKEND", "python")
    monkeypatch.setenv("DWPY_BACKEND", "python")
    monkeypatch.setattr(bridge, "RustDataWeaveRuntime", construct)
    if compiled:
        run.new_rust_runtime()
    else:
        with pytest.raises(ValueError, match="Compiled Rust bridge required"):
            run.new_rust_runtime()


def test_stale_reference_expectation_fails_without_modifying_cases(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    file = case_file(tmp_path, case_text(expect="1"))
    before = file.read_bytes()
    monkeypatch.setattr(run, "new_rust_runtime", lambda: FakeRust("2"))
    monkeypatch.setattr(run, "run_engine", lambda *args: {0: "2"})
    assert run.main([str(file), "--check"]) == 1
    assert file.read_bytes() == before


class FakeNativeResult:
    def __init__(
        self,
        text: str | None,
        error: str | None = None,
        *,
        mime_type: str | None = "application/json",
    ) -> None:
        self.success = error is None
        self.error = error
        self.text = text
        self.mime_type = mime_type

    def get_string(self) -> str | None:
        return self.text


def test_reference_runs_scripts_and_records_rejection_without_guessing_phase(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    case = sample_case()
    responses = iter(
        [FakeNativeResult('{ "a": 1 }'), FakeNativeResult(None, "script rejected")]
    )
    calls = []

    def execute(script: str) -> FakeNativeResult:
        calls.append(script)
        return next(responses)

    monkeypatch.setattr(
        run, "new_reference_runtime", lambda: nullcontext(SimpleNamespace(run=execute))
    )
    messages = {}
    assert run.run_engine([case, case], messages) == {0: '{"a":1}', 1: run.ERROR}
    assert messages == {1: "script rejected"}
    assert calls == [case.script, case.script]


def test_missing_success_output_is_setup_error(monkeypatch: pytest.MonkeyPatch) -> None:
    engine = SimpleNamespace(run=lambda script: FakeNativeResult(None))
    monkeypatch.setattr(run, "new_reference_runtime", lambda: nullcontext(engine))
    with pytest.raises(ValueError, match="no output"):
        run.run_engine([sample_case()])


def test_reference_version_is_pinned(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(run, "version", lambda package: "0.0.0")
    with (
        pytest.raises(
            ValueError, match=f"Expected dataweave-native=={run.REFERENCE_VERSION}"
        ),
        run.new_reference_runtime(),
    ):
        pytest.fail("Version mismatch initialized an engine")


def test_missing_reference_package_fails_setup(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    file = case_file(tmp_path, case_text(expect="1"))
    monkeypatch.setattr(run, "new_rust_runtime", lambda: FakeRust())

    def missing(package: str) -> str:
        raise PackageNotFoundError(package)

    monkeypatch.setattr(run, "version", missing)
    assert run.main([str(file), "--check"]) == 2


def test_reference_context_cleans_up_and_wraps_binding_failures(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import sys

    lifecycle = []

    class BindingError(Exception):
        pass

    class Engine:
        def __enter__(self):
            lifecycle.append("initialized")
            return self

        def __exit__(self, *args: object) -> None:
            lifecycle.append("cleaned")

        def run(self, script: str) -> FakeNativeResult:
            raise BindingError("library failed")

    monkeypatch.setitem(
        sys.modules,
        "dataweave",
        SimpleNamespace(DataWeave=Engine, DataWeaveError=BindingError),
    )
    monkeypatch.setattr(run, "version", lambda package: run.REFERENCE_VERSION)
    with pytest.raises(ValueError, match="library failed"):
        run.run_engine([sample_case()])
    assert lifecycle == ["initialized", "cleaned"]


def test_multiline_text_expectations_round_trip(tmp_path: Path) -> None:
    text = "<?xml version='1.0'?>\n<a>1</a>\n"
    encoded = run._output(text, "application/xml")
    assert encoded.startswith('text "')
    assert "\n" not in encoded
    file = case_file(tmp_path, case_text("xml"))
    assert run.update(run.parse_cases(file), {0: encoded}) == 1
    parsed = run.parse_cases(file)[0]
    assert run.check(parsed, parsed.expect, FakeRust(text)) is None
    assert run.check(parsed, parsed.expect, FakeRust(text.rstrip())) is not None


@pytest.mark.parametrize("mime_type", ["text/plain", "application/csv", None])
@pytest.mark.parametrize(
    ("reference", "candidate", "matches"),
    [
        ("1\n", "1", False),
        ("1\r\n", "1\n", False),
        ("1.0", "1", False),
        ('{"a":1}', '{ "a": 1 }', False),
        ("1\n", "1\n", True),
    ],
)
def test_json_looking_text_is_compared_exactly(
    monkeypatch: pytest.MonkeyPatch,
    mime_type: str | None,
    reference: str,
    candidate: str,
    matches: bool,
) -> None:
    engine = SimpleNamespace(
        run=lambda script: FakeNativeResult(reference, mime_type=mime_type)
    )
    monkeypatch.setattr(run, "new_reference_runtime", lambda: nullcontext(engine))
    case = sample_case()
    expected = run.run_engine([case])[0]
    assert expected.startswith('text "')
    assert (run.check(case, expected, FakeRust(candidate)) is None) is matches


@pytest.mark.parametrize(
    "mime_type",
    ["application/json", "Application/JSON; charset=UTF-8", "application/problem+json"],
)
def test_declared_json_retains_semantic_comparison(
    monkeypatch: pytest.MonkeyPatch, mime_type: str
) -> None:
    engine = SimpleNamespace(
        run=lambda script: FakeNativeResult('{ "a": 1.0 }\n', mime_type=mime_type)
    )
    monkeypatch.setattr(run, "new_reference_runtime", lambda: nullcontext(engine))
    case = sample_case()
    expected = run.run_engine([case])[0]
    assert run.check(case, expected, FakeRust('{"a":1}')) is None


def test_malformed_json_output_is_not_reinterpreted_as_text(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    case = sample_case()
    assert "Rust returned invalid JSON" in run.check(case, '{"a":1}', FakeRust('{"a":'))
    engine = SimpleNamespace(run=lambda script: FakeNativeResult('{"a":'))
    monkeypatch.setattr(run, "new_reference_runtime", lambda: nullcontext(engine))
    with pytest.raises(ValueError):
        run.run_engine([case])


def test_offline_comparison_retains_numeric_text_newline(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    expected = run._output("1\n", "text/plain")
    file = case_file(tmp_path, case_text("text", expected))
    monkeypatch.setattr(run, "new_rust_runtime", lambda: FakeRust("1"))
    assert run.main([str(file), "--offline", "--check"]) == 1


def test_json_report_identifies_each_case_and_reproduction_command(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    import json

    file = case_file(
        tmp_path, case_text("works", "1") + case_text("differs", "2")
    )
    monkeypatch.setattr(run, "new_rust_runtime", lambda: FakeRust("1"))
    report = tmp_path / "results/report.json"
    assert run.main([str(file), "--offline", "--check", "--report", str(report)]) == 1
    result = json.loads(report.read_text())
    assert result["summary"] == {
        "total": 2,
        "passed": 1,
        "failed": 1,
        "reference_failures": 0,
    }
    assert result["reference"] == {
        "package": "dataweave-native",
        "version": run.REFERENCE_VERSION,
        "live": False,
    }
    failure = result["cases"][1]
    assert failure["id"] == "cases/differs"
    assert failure["status"] == "fail"
    assert failure["detail"]
    assert failure["reproduce"][-2:] == ["--check", "--offline"]
    assert failure["reproduce"][failure["reproduce"].index("--case") + 1] == "differs"


@pytest.mark.parametrize(
    "diagnostic",
    [
        "Native returned empty response",
        "Native returned null",
        "Failed to parse native JSON response: Expecting value",
        "Native response JSON is not an object",
        "",
        None,
    ],
)
def test_binding_failures_are_not_script_rejections(
    monkeypatch: pytest.MonkeyPatch,
    diagnostic: str | None,
) -> None:
    result = FakeNativeResult(None, diagnostic)
    result.success = False
    engine = SimpleNamespace(
        run=lambda script: result
    )
    monkeypatch.setattr(run, "new_reference_runtime", lambda: nullcontext(engine))
    messages = {}
    assert run.run_engine([sample_case()], messages) == {0: run.REFERENCE_FAILURE}
    assert messages[0] == (diagnostic or "Reference failed without a diagnostic")


@pytest.mark.parametrize("offline", [False, True])
def test_unverified_reference_does_not_run_rust_or_establish_an_expectation(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, offline: bool
) -> None:
    file = case_file(tmp_path, case_text())

    class MustNotExecute(FakeRust):
        def execute(self, script_source: str, payload: object) -> object:
            pytest.fail("An unavailable reference was used to judge Rust")

    monkeypatch.setattr(run, "new_rust_runtime", MustNotExecute)
    monkeypatch.setattr(run, "run_engine", lambda *args: {0: run.REFERENCE_FAILURE})
    args = [str(file), "--check", "--offline" if offline else "--update"]
    assert run.main(args) == 1
    assert "// expect ?\n" in file.read_text()


@pytest.mark.parametrize("expect", ["?", "1", run.ERROR])
def test_reference_outage_preserves_expectations_and_skips_rust(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, expect: str
) -> None:
    import json

    file = case_file(tmp_path, case_text(expect=expect))
    before = file.read_bytes()

    class MustNotExecute(FakeRust):
        def execute(self, script_source: str, payload: object) -> object:
            pytest.fail("Reference outages must not judge Rust")

    monkeypatch.setattr(run, "new_rust_runtime", MustNotExecute)
    engine = SimpleNamespace(
        run=lambda script: FakeNativeResult(None, "Native returned null")
    )
    monkeypatch.setattr(run, "new_reference_runtime", lambda: nullcontext(engine))
    report = tmp_path / "report.json"
    assert run.main([str(file), "--update", "--check", "--report", str(report)]) == 1
    assert file.read_bytes() == before
    record = json.loads(report.read_text())["cases"][0]
    assert record["expected"] == expect
    assert record["status"] == "reference-error"


def test_update_records_successes_while_preserving_failed_reference_cases(
    tmp_path: Path,
) -> None:
    file = case_file(tmp_path, case_text("unavailable", "1") + case_text("verified"))
    assert run.update(run.parse_cases(file), {0: run.REFERENCE_FAILURE, 1: "2"}) == 1
    assert [case.expect for case in run.parse_cases(file)] == ["1", "2"]
