const test = require("node:test");
const assert = require("node:assert/strict");
const cases = require("../../../tests/fixtures/dependency-analysis/gaps.json");
const { analyze, execute } = require("../index.js");

for (const item of cases) {
  test(`dependency analysis agrees with execution: ${item.id}`, () => {
    assert.deepEqual(execute({ ...item.request, script: item.request.expression, render_output: false }), item.expectedResult);
    const result = analyze(item.request);
    assert.deepEqual(result.references, item.expectedReferences);
    assert.deepEqual(result.unresolvedReferences, item.expectedUnresolvedReferences);
  });
}

test("scope boundaries and string contents do not hide real dependencies", () => {
  for (const [expression, references, unresolved] of [
    ["[payload map (item) -> item.price, item.price]", ["item.price", "payload"], ["item.price"]],
    ["[using (x = record.a) x, x]", ["record.a", "x"], ["record.a", "x"]],
    ['"text record.fake $(record.real)"', ["record.real"], ["record.real"]],
    ["[update, mask, replace, matches, with, String]", ["String", "mask", "matches", "replace", "update", "with"], ["String", "mask", "matches", "replace", "update", "with"]],
  ]) {
    const result = analyze({ expression, payload: {} });
    assert.deepEqual(result.references, references, expression);
    assert.deepEqual(result.unresolvedReferences, unresolved, expression);
  }
});

test("invalid syntax stays an analysis error and headers are never evaluated", () => {
  for (const expression of ['"unfinished', "{a 1}", "payload +", "payload["]) {
    assert.throws(() => analyze({ expression, payload: {} }), /parse error|Missing Expression/i);
  }
  const result = analyze({ expression: '%dw 2.0\nvar unused = fail("must not run")\n---\npayload.value', payload: { value: 1 } });
  assert.deepEqual(result.references, ["payload.value"]);
});

test("declared inputs retain their underlying payload or vars dependency", () => {
  const primary = analyze({ expression: "%dw 2.0\ninput payload application/json\n---\npayload.name", payload: {name: "ana"} });
  assert.deepEqual(primary.references, ["payload.name"]);
  assert.deepEqual(primary.unresolvedReferences, []);
  const named = analyze({ expression: "%dw 2.0\ninput data application/json\n---\ndata.name", vars: {data: {name: "ana"}} });
  assert.deepEqual(named.references, ["vars.data"]);
  assert.deepEqual(named.unresolvedReferences, []);
});
