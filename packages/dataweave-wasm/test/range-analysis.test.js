const test = require("node:test");
const assert = require("node:assert/strict");
const { analyze, execute } = require("../index.js");

for (const [expression, expected] of [
  ["1 to 10 map $", [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]],
  ["{values: (1 to 3 map ($ * 2))}", { values: [2, 4, 6] }],
  ["(1\n/* bounds */ to\n3) map $", [1, 2, 3]],
  ["(3 to -1) map $", [3, 2, 1, 0, -1]],
  ["[1, 2, 3][0 to 1]", [1, 2]],
  ["7 mod 4", 3],
  ["2 pow 3", 8],
]) {
  test(`range and numeric syntax passes formula analysis: ${expression}`, () => {
    const analysis = analyze({ expression, payload: {} });
    assert.deepEqual(analysis.references, []);
    assert.deepEqual(analysis.unresolvedReferences, []);
    assert.deepEqual(execute({ script: expression, render_output: false }), expected);
  });
}

test("range bounds preserve real Deal API formula dependencies", () => {
  const expression = "record.start to record.end map $";
  const payload = { record: { start: 1, end: 3 } };
  const analysis = analyze({ expression, payload });
  assert.deepEqual(analysis.references, ["record.end", "record.start"]);
  assert.deepEqual(analysis.unresolvedReferences, []);
  const script = `%dw 2.0\noutput application/python\nvar record = payload.record\n---\n${expression}`;
  assert.deepEqual(execute({ script, payload, render_output: false }), [1, 2, 3]);
});

for (const [expression, references] of [
  ["to default 3", ["to"]],
  ["[to, mod, pow]", ["mod", "pow", "to"]],
  ["if (true) to else 3", ["to"]],
  ["1 to missing", ["missing"]],
  ["payload.to", ["payload.to"]],
]) {
  test(`range analysis still reports missing fields: ${expression}`, () => {
    const analysis = analyze({ expression, payload: {} });
    assert.deepEqual(analysis.references, references);
    assert.deepEqual(analysis.unresolvedReferences, references);
  });
}
