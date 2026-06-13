from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Optional, Sequence

from ._cli_shared import (
    infer_payload_format_from_path,
    parse_inline_payload,
    render_cli_value,
    script_renders_text,
)
from .runtime import DataWeaveEvaluationError, DataWeaveRuntime


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="dw")
    subparsers = parser.add_subparsers(dest="command", required=True)

    run_parser = subparsers.add_parser("run", help="Run a DataWeave script")
    script_group = run_parser.add_mutually_exclusive_group(required=True)
    script_group.add_argument("script", nargs="?", help="Inline DataWeave script source")
    script_group.add_argument("--file", dest="script_file", help="Path to a DataWeave script")

    payload_group = run_parser.add_mutually_exclusive_group()
    payload_group.add_argument("--payload", help="Inline payload value")
    payload_group.add_argument("--payload-file", help="Path to a payload file")
    run_parser.add_argument(
        "--payload-format",
        help="Payload mime type, for example application/json or application/xml",
    )
    run_parser.set_defaults(handler=run_command)
    return parser


def main(argv: Optional[Sequence[str]] = None) -> int:
    args = build_parser().parse_args(argv)
    return args.handler(args)


def run_command(args: argparse.Namespace) -> int:
    try:
        script_source = _read_script_source(args)
        payload, payload_format = _load_payload(args)
        runtime = _strict_rust_runtime()
        result = runtime.execute(
            script_source,
            payload,
            payload_format=payload_format,
        )
    except FileNotFoundError as err:
        _print_error(str(err))
        return 1
    except RuntimeError as err:
        _print_error(str(err))
        return 1
    except (DataWeaveEvaluationError, NotImplementedError, ValueError) as err:
        _print_error(str(err))
        return 1

    if script_renders_text(script_source) and isinstance(result, str):
        sys.stdout.write(result)
    else:
        sys.stdout.write(render_cli_value(result))
        sys.stdout.write("\n")
    return 0


def _read_script_source(args: argparse.Namespace) -> str:
    if args.script_file:
        return Path(args.script_file).read_text(encoding="utf-8")
    return args.script


def _load_payload(args: argparse.Namespace) -> tuple[object, Optional[str]]:
    if args.payload is not None:
        if args.payload_format:
            return args.payload, args.payload_format
        return parse_inline_payload(args.payload), None

    if args.payload_file is not None:
        payload_text = Path(args.payload_file).read_text(encoding="utf-8")
        inferred_format = args.payload_format or infer_payload_format_from_path(args.payload_file)
        if inferred_format is None:
            return payload_text, None
        return payload_text, inferred_format

    return {}, None


def _strict_rust_runtime() -> DataWeaveRuntime:
    runtime = DataWeaveRuntime(backend="rust")
    capabilities = set(runtime.capabilities())
    if "source-checkout-fallback" in capabilities:
        raise RuntimeError(
            "Rust-only CLI requires the compiled native backend. Build or install the Rust extension before running `dw`."
        )
    return runtime


def _print_error(message: str) -> None:
    sys.stderr.write(f"{message}\n")


if __name__ == "__main__":
    raise SystemExit(main())
