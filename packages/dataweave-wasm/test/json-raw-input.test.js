const test = require("node:test");
const assert = require("node:assert/strict");
const { spawnSync } = require("node:child_process");
const path = require("node:path");
const bindings = require("../dist/dwpy_wasm.js");
const { execute, analyze } = require("../index.js");

const payload = '{"test":{"test2":{"array":1},"test2":{"array":2},"test2":{"array":3}}}';
const script = "%dw 2.0\noutput application/json\n---\npayload.test.*test2";
const request = '{"script":' + JSON.stringify(script) + ',"payload":' + payload + '}';
const expected = [{ array: 1 }, { array: 2 }, { array: 3 }];

test("WASM smoke entry point preserves duplicate keys in raw payload JSON", () => {
  assert.deepEqual(JSON.parse(bindings.run_dataweave_smoke(script, payload)), expected);
});

test("WASM request entry point preserves duplicate keys in inline payload JSON", () => {
  const output = JSON.parse(bindings.run_dataweave_request(request));
  assert.deepEqual(JSON.parse(output), expected);
});

test("WASM request preserves duplicate keys in vars and attributes", () => {
  const source = '{"script":"{values: vars.*a, headers: attributes.*h}","vars":{"a":1,"a":2},"attributes":{"h":"first","h":"last"}}';
  assert.deepEqual(JSON.parse(bindings.run_dataweave_request(source)), {
    values: [1, 2], headers: ["first", "last"],
  });
});

test("Node execute accepts serialized requests without discarding duplicates", () => {
  assert.deepEqual(JSON.parse(execute(request)), expected);
});

test("Node analyze accepts serialized requests", () => {
  const result = analyze('{"expression":"payload.test","payload":' + payload + '}');
  assert.deepEqual(result.references, ["payload.test"]);
  assert.deepEqual(result.unresolvedReferences, []);
});

test("stdio forwards the original request JSON to WASM", () => {
  const output = spawnSync(process.execPath, [path.join(__dirname, "../stdio.js")], {
    input: request + "\n", encoding: "utf8",
  });
  assert.equal(output.status, 0, output.stderr);
  const response = JSON.parse(output.stdout);
  assert.equal(response.ok, true, response.error);
  assert.deepEqual(JSON.parse(response.result), expected);
});

test("stdio still dispatches serialized analysis requests", () => {
  const source = '{"operation":"analyze","expression":"payload.test","payload":' + payload + '}';
  const output = spawnSync(process.execPath, [path.join(__dirname, "../stdio.js")], {
    input: source + "\n", encoding: "utf8",
  });
  assert.equal(output.status, 0, output.stderr);
  const response = JSON.parse(output.stdout);
  assert.equal(response.ok, true, response.error);
  assert.deepEqual(response.result.references, ["payload.test"]);
});

test("raw JSON input rejects trailing content and malformed nested values", () => {
  for (const source of [request + " trailing", '{"script":"payload","payload":{"a":1,"a":}}']) {
    assert.throws(() => bindings.run_dataweave_request(source));
  }
});

test("request metadata keeps its existing last-key behavior", () => {
  assert.deepEqual(execute('{"script":"payload","script":"payload.*a","payload":{"a":1,"a":2}}'), [1, 2]);
});

test("typed requests preserve duplicate inline execution values", () => {
  const typedRequest = request.slice(0, -1) + ',"typed_transport":true}';
  assert.deepEqual(JSON.parse(execute(typedRequest)), expected);
});
