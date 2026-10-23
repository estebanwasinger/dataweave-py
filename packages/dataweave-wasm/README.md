# @estebanwasinger/dataweave-wasm

Node.js bindings for the strict Rust DataWeave runtime.

Build with `npm run build` after installing `wasm-pack` and the
`wasm32-unknown-unknown` Rust target. The generated `dist/` directory is
included when the package is published to GitHub Packages.

Run the JavaScript API tests after building:

```bash
npm test
```

Run the Rust unit and integration tests compiled to WebAssembly, using
`wasm-bindgen-test` in Node.js:

```bash
npm run test:rust
npm run test:rust -- -- infix
```

These tests cover the WASM entry points, including dependency analysis before
execution. They require the build prerequisites above and do not require Python.
They test the Node.js host; Deno and browser integration need separate checks.
