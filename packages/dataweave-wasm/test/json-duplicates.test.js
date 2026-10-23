const test = require("node:test");
const assert = require("node:assert/strict");
const { execute } = require("../index.js");

const payload = '{"test":{"test2":{"array":1},"test2":{"array":2},"test2":{"array":3}}}';

test("JSON payload preserves duplicate keys for multi-value selectors", () => {
  const result = execute({
    script: "%dw 2.0\noutput application/json\n---\npayload.test.*test2",
    payload,
    payload_format: "application/json",
  });
  assert.deepEqual(JSON.parse(result), [{ array: 1 }, { array: 2 }, { array: 3 }]);
});

test("read JSON preserves duplicate keys in composed expressions", () => {
  assert.deepEqual(execute({
    script: '(read(payload, "application/json").test.*test2) map $.array',
    payload,
  }), [1, 2, 3]);
});
