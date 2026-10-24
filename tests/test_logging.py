import logging
import os
from concurrent.futures import ThreadPoolExecutor

import pytest

from dwpy import DataWeaveRuntime, LogEntry


BACKENDS = ["python", "rust", "wasm"]


@pytest.mark.parametrize("backend", BACKENDS)
def test_injected_loggers_receive_every_map_value_without_changing_output(backend):
    first, second = [], []
    runtime = DataWeaveRuntime(backend=backend, loggers=[first.append, second.append])
    result = runtime.execute("%dw 2.0\noutput application/json\n---\n1 to 10 map log($)", None)
    import json
    assert json.loads(result) == list(range(1, 11))
    assert first == second == [LogEntry("warn", None, str(n)) for n in range(1, 11)]


@pytest.mark.parametrize("backend", BACKENDS)
def test_logging_levels_nested_calls_and_headers(backend):
    entries = []
    runtime = DataWeaveRuntime(backend=backend, loggers=[entries.append])
    result = runtime.execute('''%dw 2.0
var start = logDebug("header", 1)
fun inspect(value) = logError(log("prefix", "outer"), logInfo("inner", value))
---
[inspect(start), logWarn(2), logWith("DEBUG", "custom", 3)]''', None)
    assert result == [1, 2, 3]
    assert entries == [
        LogEntry("debug", "header", "header - 1"),
        LogEntry("warn", "prefix", "prefix - outer"),
        LogEntry("info", "inner", "inner - 1"),
        LogEntry("error", "outer", "outer - 1"),
        LogEntry("warn", None, "2"),
        LogEntry("debug", "custom", "custom - 3"),
    ]


@pytest.mark.parametrize("backend", BACKENDS)
def test_logs_before_error_are_delivered_and_next_run_is_isolated(backend):
    entries = []
    runtime = DataWeaveRuntime(backend=backend, loggers=[entries.append])
    with pytest.raises(Exception):
        runtime.execute('import fail from dw::Runtime\noutput application/json\n---\n[log("before"), fail("boom")]', None)
    assert entries == [LogEntry("warn", None, "before")]
    DataWeaveRuntime(backend=backend, loggers=[]).execute('log("other")', None)
    assert runtime.execute('log("after")', None) == "after"
    assert [entry.message for entry in entries] == ["before", "after"]


@pytest.mark.parametrize("backend", BACKENDS)
def test_python_logger_uses_its_level_and_does_not_duplicate_to_root(backend, caplog):
    logger = logging.getLogger("dwpy.test.injected")
    logger.setLevel(logging.INFO)
    with caplog.at_level(logging.DEBUG):
        DataWeaveRuntime(backend=backend, loggers=[logger]).execute('[logDebug("hidden"), logInfo("shown")]', None)
    assert [(r.name, r.levelname, r.message) for r in caplog.records] == [(logger.name, "INFO", "shown")]


@pytest.mark.parametrize("backend", BACKENDS)
def test_callback_exceptions_propagate_and_scopes_restore(backend):
    def broken(entry):
        raise RuntimeError("sink failed")
    with pytest.raises(Exception, match="sink failed"):
        DataWeaveRuntime(backend=backend, loggers=[broken]).execute('log(1)', None)
    entries = []
    DataWeaveRuntime(backend=backend, loggers=[entries.append]).execute('log(2)', None)
    assert entries == [LogEntry("warn", None, "2")]


@pytest.mark.skipif(os.environ.get("DWPY_TEST_BACKEND") is not None, reason="auto selection is tested outside the forced-backend matrix")
def test_auto_fallback_does_not_deliver_logs_from_both_attempts():
    entries = []
    runtime = DataWeaveRuntime(backend="auto", loggers=[entries.append])
    # The second function is Python-only, so Rust falls back after logging.
    assert runtime.execute('[log("once"), isLegacySizeOfNumber()]', None) == ["once", False]
    assert runtime._rust_runtime.last_execution_engine() == "python-legacy"
    assert entries == [LogEntry("warn", None, "once")]


@pytest.mark.parametrize("backend", BACKENDS)
def test_concurrent_engines_and_reentrant_sinks_are_isolated(backend):
    def run(value):
        outer, inner = [], []
        def nested(entry):
            DataWeaveRuntime(backend=backend, loggers=[inner.append]).execute('log("inner")', None)
        runtime = DataWeaveRuntime(backend=backend, loggers=[outer.append, nested])
        assert runtime.execute('log(payload)', value) == value
        return outer, inner
    with ThreadPoolExecutor(max_workers=3) as pool:
        for value, (outer, inner) in enumerate(pool.map(run, range(3))):
            assert outer == [LogEntry("warn", None, str(value))]
            assert inner == [LogEntry("warn", None, "inner")]


def test_logger_configuration_is_validated():
    with pytest.raises(TypeError, match="loggers must contain"):
        DataWeaveRuntime(loggers=[object()])


@pytest.mark.parametrize("backend", BACKENDS)
def test_explicit_sinks_replace_default_logging_and_empty_list_silences_it(backend, caplog):
    entries = []
    with caplog.at_level(logging.DEBUG):
        DataWeaveRuntime(backend=backend, loggers=[entries.append]).execute('log("captured")', None)
        DataWeaveRuntime(backend=backend, loggers=[]).execute('log("silent")', None)
    assert [entry.message for entry in entries] == ["captured"]
    assert not caplog.records


@pytest.mark.parametrize("backend", ["rust", "wasm"])
def test_log_messages_render_duplicate_fields_without_wire_markers(backend):
    if os.environ.get("DWPY_TEST_BACKEND") == "python":
        pytest.skip("native message formatting contract")
    entries = []
    result = DataWeaveRuntime(backend=backend, loggers=[entries.append]).execute(
        'log(payload).*a', '{"a":1,"a":2}', payload_format="application/json")
    assert result == [1, 2]
    assert entries[0].message == '{"a":1,"a":2}'
