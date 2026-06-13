from __future__ import annotations

import json
from pathlib import Path

import pytest

from dwpy import cli


def test_cli_runs_inline_script_and_renders_native_values_as_pretty_json(
    capsys: pytest.CaptureFixture[str],
) -> None:
    exit_code = cli.main(
        [
            "run",
            "%dw 2.0\noutput application/python\n---\n{message: upper(payload.name)}",
            "--payload",
            '{"name":"dw"}',
        ]
    )

    captured = capsys.readouterr()

    assert exit_code == 0
    assert json.loads(captured.out) == {"message": "DW"}
    assert captured.err == ""


def test_cli_runs_script_file_with_inferred_payload_file_format(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    script_path = tmp_path / "script.dwl"
    payload_path = tmp_path / "payload.json"
    script_path.write_text("%dw 2.0\noutput application/json\n---\npayload.user", encoding="utf-8")
    payload_path.write_text('{"user":{"name":"Ana"}}', encoding="utf-8")

    exit_code = cli.main(
        [
            "run",
            "--file",
            str(script_path),
            "--payload-file",
            str(payload_path),
        ]
    )

    captured = capsys.readouterr()

    assert exit_code == 0
    assert json.loads(captured.out) == {"name": "Ana"}
    assert captured.err == ""


def test_cli_prints_rendered_text_output_as_is(
    capsys: pytest.CaptureFixture[str],
) -> None:
    exit_code = cli.main(
        [
            "run",
            "%dw 2.0\noutput text/plain\n---\nupper(payload.name)",
            "--payload",
            '{"name":"dw"}',
        ]
    )

    captured = capsys.readouterr()

    assert exit_code == 0
    assert captured.out == "DW"
    assert captured.err == ""


def test_cli_reports_usage_error_for_multiple_script_sources() -> None:
    with pytest.raises(SystemExit) as exc_info:
        cli.main(["run", "payload", "--file", "script.dwl"])

    assert exc_info.value.code == 2


def test_cli_rejects_unsupported_features_in_strict_rust_mode(
    capsys: pytest.CaptureFixture[str],
) -> None:
    exit_code = cli.main(
        [
            "run",
            "%dw 2.0\noutput application/json\n---\npayload",
            "--payload",
            '{"name":"dw"}',
            "--payload-format",
            "application/avro",
        ]
    )

    captured = capsys.readouterr()

    assert exit_code == 1
    assert captured.out == ""
    assert "unsupported" in captured.err.lower() or "Rust" in captured.err
