# Repository Guidelines

# New Python Project (Current Development)

## Project Structure
- `dwpy/`: Primary package hosting the Python runtime; key modules include `runtime.py` (execution entry points), `type_inference.py`/`typesystem.py` (static typing helpers), `builtins.py` plus `modules/**/*.dwl` (built-in DataWeave modules), and `pydantic_export.py` for schema interop.
- `tests/`: Pytest suites mirroring user-level features—add new specs here alongside any fixture data.
- `docs/`: Markdown and reference material for the Python port; extend when introducing new features or CLI surfaces.
- `dist/` & `dataweave_py.egg-info/`: Generated packaging artifacts/metadata; regenerated via `uv run -- python -m build`.
- `runtime-2.11.0-20250825*/`: Upstream Scala distribution kept for reference/comparison; do not edit unless syncing from the JVM source bundle.
- Repo root files (`pyproject.toml`, `uv.lock`, `README.md`, `AGENTS.md`) define build config, dependency locks, and contributor guidance—update them when altering workflows.

## Versioning
- Prefer `uv version <patch|minor|major|X.Y.Z>` to update `pyproject.toml`; it keeps SemVer consistent and aligns with uv tooling already in use.
- Run `uv lock` after bumping so the lockfile captures the new version (or trigger the `Bump Version` workflow, which performs both steps and pushes the commit).

## Data Formats
- `dwpy/runtime.py` now understands structured I/O formats through `FormatRegistry`; pass `payload_format`/`payload_format_options` (for example `payload_format="application/csv", payload_format_options={"separator": ";"}` or `payload_format="application/xml"`) so raw text payloads are parsed before evaluation.
- Output directives are honoured: `output application/json` returns JSON text, `output application/csv` returns CSV text, and `output application/python` (or omitting the directive) yields raw Python objects; set `render_output=False` when calling `DataWeaveRuntime.execute` to bypass serialization for tests or REPL work.
- Writer properties from the header (e.g., `output application/csv separator=";" header=false`) are parsed and forwarded to the corresponding writer; JSON accepts `indent`, `ensure_ascii`, `sort_keys`, and CSV supports `separator`, `quote`, `header`, `columns`, etc.

## Language Feature Implementation Rules
When adding new DataWeave language features or semantics, treat parser/runtime behavior as a single contract:

- Implement grammar features in the parser proper, not by ad-hoc string preprocessing shortcuts when the feature can appear as a nested expression.
- Keep script/header parsing delimiter-aware and comment-aware; avoid naive `split("---")` or other approaches that ignore nesting/comments.
- Add or update AST nodes in `dwpy/parser.py` and evaluate them in `dwpy/runtime.py` with explicit scoping rules.
- Reuse common header execution logic instead of duplicating function/var/import handling across code paths.
- Preserve compatibility: existing scripts and tests must keep passing after feature changes.
- Add typing support when relevant: update `dwpy/type_inference.py` and `dwpy/typesystem.py` for new expression forms or coercion rules.
- Add focused regression tests for:
  - happy-path behavior,
  - nesting/composition behavior,
  - parser boundary cases (comments, multiline, delimiters),
  - failure/error messaging where applicable.
- Prefer real DataWeave cookbook/spec-inspired examples in tests when possible.
- If behavior intentionally diverges from MuleSoft runtime, document the limitation in tests and docs.
- Any parser/runtime feature PR is incomplete unless tests are added and pass with:
  - `UV_CACHE_DIR=.uv-cache uv run --extra dev pytest`

# Running Tests
Execute the Python test suite with uv so dependencies resolve automatically:
```
UV_CACHE_DIR=.uv-cache uv run --extra dev pytest
```
The `--extra dev` flag installs the dev dependencies defined in `pyproject.toml`, while `UV_CACHE_DIR` keeps uv's cache local to the repo for environments with restricted access.

# Old Java Project (Legacy)

## Project Structure & Module Organization
This source bundle represents the DataWeave `runtime` module. Primary code resides in `org/mule/weave/v2`, with subpackages such as `runtime` (execution APIs), `lang` (language constructs), `compilation` (loader and optimizer pipeline), and `interpreted` (on-demand evaluation helpers). Documentation assets for built-in data formats live in `data-format/dw`, while service wiring and native-image metadata are stored under `META-INF/services` and `META-INF/native-image` respectively. Keep new types aligned with the existing package tree and update the relevant service descriptor when introducing injectable components.

## Coding Style & Naming Conventions
Author code in Scala 2.12 and let the Gradle Scala plugin emit the Java stubs distributed here. Use two-space indentation, curried method signatures where helpful, and prefer immutable collections. Follow package-aligned names (e.g., `ModuleComponentsFactory` in `org.mule.weave.v2.runtime`). Public APIs should expose idiomatic Scala types (`Option`, `Try`) and convert to Java-friendly facades only at the boundary. Update `META-INF/services` when adding SPI implementations and keep companion objects in the same file as their classes.

## Testing Guidelines
Tests live in the main repository under `runtime/src/test/scala`. Mirror the production package hierarchy and suffix suites with `Spec` (property-based) or `Test` (example-driven). Exercise both interpreted and compiled execution paths, adding regression data where appropriate. Run `./gradlew :runtime:test` before opening a pull request; include new fixtures or golden files alongside assertions.

## Commit & Pull Request Guidelines
Write Conventional Commit messages (`type(scope): summary`) that reference the affected module, for example `fix(runtime): guard null stream in ExecuteResult`. Each pull request should describe the change, link to Jira/GitHub issues, and call out observable behaviour shifts. Attach screenshots or stack traces when touching tooling, regenerate docs when necessary, and request reviews from maintainers responsible for the impacted package.

## Native Image & Packaging Notes
When altering startup or reflection behaviour, update the corresponding JSON under `META-INF/native-image/org.mule.weave/runtime`. Ensure new entry points remain discoverable via `META-INF/services` so downstream runtimes pick them up during assembly. Publish snapshot artifacts only after verifying the jar manifests retain `Automatic-Module-Name: org.mule.weave.runtime`.
