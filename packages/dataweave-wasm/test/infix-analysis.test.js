const test = require("node:test");
const assert = require("node:assert/strict");
const { analyze, execute } = require("../index.js");

for (const expression of [
  '[1,2,3] joinBy ", "',
  'joinBy([1,2,3], ", ")',
  '[1,2,3] joinBy(", ")',
  '([1,2,3]\n/* separator */ joinBy ", ")',
  '{default: [1,2,3]}.default joinBy ", "',
]) {
  test(`analyzes and executes ${expression}`, () => {
    const analysis = analyze({ expression, payload: {} });
    assert.deepEqual(analysis.references, []);
    assert.deepEqual(analysis.unresolvedReferences, []);
    assert.equal(execute({ script: expression }), "1, 2, 3");
  });
}

test("infix functions preserve actual formula dependencies", () => {
  const expression = '(record.names joinBy ", ") splitBy record.separator';
  const payload = { record: { names: ["a", "b"], separator: ", " } };
  const analysis = analyze({ expression, payload });
  assert.deepEqual(analysis.references, ["record.names", "record.separator"]);
  assert.deepEqual(analysis.unresolvedReferences, []);
  const script = `%dw 2.0\nvar record = payload.record\n---\n${expression}`;
  assert.deepEqual(execute({ script, payload, render_output: false }), ["a", "b"]);
});

test("analysis still reports missing fields beside infix functions", () => {
  const analysis = analyze({
    expression: 'payload.missing joinBy ", "',
    payload: {},
  });
  assert.deepEqual(analysis.references, ["payload.missing"]);
  assert.deepEqual(analysis.unresolvedReferences, ["payload.missing"]);
});

for (const [expression, references] of [
  ["joinBy", ["joinBy"]],
  ["[joinBy, splitBy]", ["joinBy", "splitBy"]],
  ["payload.names default joinBy", ["joinBy", "payload.names"]],
  ["joinBy ++ splitBy", ["joinBy", "splitBy"]],
  ["if (true) joinBy else splitBy", ["joinBy", "splitBy"]],
]) {
  test(`analysis preserves function names used as fields: ${expression}`, () => {
    const analysis = analyze({ expression, payload: {} });
    assert.deepEqual(analysis.references, references);
    assert.deepEqual(analysis.unresolvedReferences, references);
  });
}
