const test = require("node:test");
const assert = require("node:assert/strict");
const { executeWithLogs, analyze } = require("..");

test("typed functions coerce compatible arguments and concatenate numbers", () => {
  const script = '%dw 2.0\noutput application/json\nfun hi(a: Number, b: Number) = a ++ b\n---\nhi(2,"3")';
  assert.equal(executeWithLogs({ script }).result, '"23"');
  const typed = '%dw 2.0\nfun add(a: Number, b: Number) = b + a\n---\nadd(2,"3")';
  assert.equal(executeWithLogs({ script: typed }).result, 5);
  assert.equal(analyze({ expression: "2 ++ 3" }).inferredType.kind, "String");
});

test("exact overloads win over coercion regardless of declaration order", () => {
  for (const definitions of [
    'fun f(x: Number) = "number"\nfun f(x: String) = "string"',
    'fun f(x: String) = "string"\nfun f(x: Number) = "number"',
  ]) {
    const script = `%dw 2.0\n${definitions}\n---\n[f("3"), f(3)]`;
    assert.deepEqual(executeWithLogs({ script }).result, ["string", "number"]);
  }
});

test("bad coercions fail without executing a body or evaluating arguments twice", () => {
  const script = '%dw 2.0\nfun f(x: Number) = fail("body executed")\n---\nf(log("not a number"))';
  const result = executeWithLogs({ script });
  assert.match(result.error, /no matching overload/);
  assert.doesNotMatch(result.error, /body executed/);
  assert.equal(result.logs.length, 1);
});

test("coercion does not permit object and string concatenation", () => {
  const result = executeWithLogs({ script: '{} ++ "_"' });
  assert.match(result.error, /function '\+\+'[\s\S]*Object[\s\S]*String/);
});
