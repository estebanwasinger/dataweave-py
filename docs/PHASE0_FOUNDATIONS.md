# Phase 0 – Foundations

## Target Python & Platform Support
- **Python versions**: 3.10, 3.11, 3.12. These cover currently supported CPython releases and align with the MuleSoft LTS window while giving us access to structural pattern matching if needed.
- **Primary platforms**: Linux x86_64 (Ubuntu 22.04 LTS), macOS (arm64 + x86_64), Windows x86_64. These match the environments where the JVM runtime runs today and where Mule runtimes will embed the interpreter.
- **Tooling**: Maintain UV-managed virtual environments; ensure CI images provide the three Python versions for parity testing.

## JVM Runtime Surface Inventory
- **Core execution APIs** (files under `runtime-2.11.0-20250825/org/mule/weave/v2/runtime`):
  `DataWeaveScriptingEngine`, `WeaveCompiler`, `RuntimeModuleCompiler`, `ExecuteResult`, `ModuleComponentsFactory`, `CompilationConfig`, `ParserConfiguration`.
- **Value Model**: `DataWeaveValue` and companions (`SimpleDataWeaveValue`, `ObjectDataWeaveValue`, `ArrayDataWeaveValue`, `SchemaValue`, `BindingValue`).
- **Interoperability Helpers**: `WeaveInput`, `DataWeaveResult`, `DataWeaveNameValue`, `ScriptingBindings`, `ExecutableWeaveHelper`.
- **Compiler/Validation**: `CompilationResult`, `ValidationResult`, `ValidationConfiguration`, `ValidationMessage`, `ValidationPhase`.
- **Service descriptors** (`runtime-2.11.0-20250825/META-INF/services`):
  - `org.mule.weave.v2.model.values.NativeValueProvider`
  - `org.mule.weave.v2.module.DataFormat`
  - `org.mule.weave.v2.parser.phase.ModuleLoader`
- **Native-image metadata** (`runtime-2.11.0-20250825/META-INF/native-image/org.mule.weave/runtime`):
  `resource-config.json`, `reflect-config.json`, `jni-config.json`, `proxy-config.json`.
- **Manifest**: `META-INF/MANIFEST.MF` exposes the module name `org.mule.weave.runtime`.

## Feature & Behaviour Requirements (Initial Draft)
- Preserve DataWeave language semantics: module imports, selectors, expressions, typed value model, and null/default handling.
- Maintain compatibility with DataWeave standard library functions, starting with core string/number/date modules.
- Provide deterministic error reporting with source locations comparable to the JVM runtime.
- Support streaming workloads and large payload processing, with graceful degradation when Python memory limits are hit.
- Ensure embedding APIs expose compile+execute flows, configurable parser and runtime options, and sandboxable evaluation contexts.

## Packaging & Distribution Approach
- **Package name**: `dwpy` (already seeded).
- **Build backend**: adopt `setuptools` via `pyproject.toml`, with UV handling dependency resolution; evaluate publishing via PyPI and internal artifact repositories.
- **Artifact strategy**: ship pure-Python wheels (`py3-none-any`), plus optional native wheels if we later wrap accelerated extensions.
- **Versioning**: mirror DataWeave runtime releases (`2.11.x`) with optional `.post` markers for Python-specific patches.
- **Entry points**: expose CLI hooks for script execution (`dwpy run script.dwl payload.json`) and Python API entry points (`DataWeaveRuntime.execute`).

## Open Questions
- Need confirmation on minimum Windows build requirements (MSVC runtime versions).
- Decide whether to bundle GraalVM-compatible metadata or drop it in favour of Python-native packaging.
- Evaluate performance targets to determine if C extensions or PyPy support are necessary.
