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

for (const expression of ["payload.test", "[payload.test]"]) {
  test(`pretty JSON output retains duplicate fields for ${expression}`, () => {
    const result = execute({
      script: "%dw 2.0\noutput application/json indent=2\n---\n" + expression,
      payload, payload_format: "application/json",
    });
    assert.equal((result.match(/"test2":/g) || []).length, 3);
    for (const value of [1, 2, 3]) assert.ok(result.includes(`"array": ${value}`));
    assert.equal(result.startsWith("["), expression.startsWith("["));
  });
}
