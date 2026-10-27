const test = require("node:test");
const assert = require("node:assert/strict");
const { executeWithLogs } = require("..");

test("execution errors retain their real cause", () => {
  const cases = [
    ["missing.foo", /Unable to evaluate expression `missing\.foo`[\s\S]*Trace:/],
    ['"no" as Number', /cannot coerce string 'no' to Number/],
    ['{} ++ "x"', /function '\+\+'[\s\S]*Object[\s\S]*String/],
    ["1 +", /Unable to evaluate expression `1 \+`/],
  ];
  for (const [script, expected] of cases) {
    const { error } = executeWithLogs({ script });
    assert.match(error, expected, script);
  }
});

test("groupBy reports the first incompatible concatenation", () => {
  const script = '%dw 2.0\nimport * from dw::core::Arrays\noutput application/json\nvar grp7 = payload groupBy ((item) -> ((item.val default "") as String) as String) ++ "_" ++ ((item.val default "") as String)\n---\ngrp7';
  const { error } = executeWithLogs({ script, payload: [{ val: "a" }] });
  assert.match(error, /function '\+\+'[\s\S]*Object[\s\S]*String/);
  assert.doesNotMatch(error, /Unable to evaluate expression/);
  assert.match(error, /4\| var grp7 =/);
  assert.match(error, /\^\^\nTrace:/);
  assert.match(error, /main::\+\+ \(line: 4, column: 84\)/);
  assert.match(error, /main::grp7 \(line: 4, column: 5\)/);
});

test("trace reports the real nested failure without repeating logs", () => {
  const script = '%dw 2.0\nfun bad(value) = value ++ "x"\noutput application/json\n---\nbad(log({a: 1}))';
  const { error, logs } = executeWithLogs({ script });
  assert.match(error, /2\| fun bad\(value\)/);
  assert.match(error, /Trace:[\s\S]*main::\+\+ \(line: 2/);
  assert.match(error, /main::bad \(line: 5/);
  assert.equal(logs.length, 1);
});

test("single-character references in the header do not hide body locations", () => {
  for (const name of ["b", "d", "w", "output"]) {
    const script = `%dw 2.0\noutput application/json\n---\n${name}`;
    const { error } = executeWithLogs({ script });
    assert.match(error, new RegExp(`Unable to evaluate expression .${name}.`));
    assert.match(error, /Trace:[\s\S]*line: 4, column: 1/);
    assert.equal((error.match(/4\|/g) || []).length, 1);
  }
});

test("a repeated identifier in a header failure points to the header", () => {
  const script = "%dw 2.0\nvar value = d\n---\nd";
  const { error } = executeWithLogs({ script });
  assert.match(error, /2\| var value = d/);
  assert.match(error, /main::main \(line: 2, column: 13\)/);
});

test("invalid range operands report the arguments and the failing operator", () => {
  const script = "%dw 2.0\noutput application/json\n---\n1 to payload";
  for (const render_output of [true, false]) {
    const { error } = executeWithLogs({ script, payload: [{ val: "a" }], render_output });
    assert.match(error, /function 'to'[\s\S]*1: Number \(1\)[\s\S]*2: Array/);
    assert.match(error, /expects arguments of these types:[\s\S]*1: Number[\s\S]*2: Number/);
    assert.match(error, /4\| 1 to payload\n     \^\^\nTrace:[\s\S]*line: 4, column: 3/);
  }
  const reversed = executeWithLogs({ script: "payload to 3", payload: [{ val: "a" }] }).error;
  assert.match(reversed, /1: Array[\s\S]*2: Number \(3\)/);
  const valid = executeWithLogs({ script: "1 to 3" });
  assert.deepEqual(valid.result, [1, 2, 3]);
});

test("optimized callbacks retain the failing expression and do not replay side effects", () => {
  for (const render_output of [true, false]) {
    const script = '%dw 2.0\noutput application/json\n---\n[1, 2] map ($ / 0)';
    const { error } = executeWithLogs({ script, render_output });
    assert.match(error, /non-finite number/);
    assert.match(error, /4\| \[1, 2\] map \(\$ \/ 0\)/);
    assert.match(error, /main::\/ \(line: 4, column: 15\)/);
  }
});

test("output and header failures retain the relevant script context", () => {
  const output = executeWithLogs({ script: '%dw 2.0\noutput text/plain\n---\n{a: 1}' }).error;
  assert.match(output, /Plain text writer expects/);
  assert.match(output, /3\| ---|4\| \{a: 1\}/);
  assert.match(output, /Trace:/);
  const header = executeWithLogs({ script: '%dw 2.0\ninput missing\n---\n1' }).error;
  assert.match(header, /expected input/);
  assert.match(header, /2\| input missing[\s\S]*Trace:/);
});

test("empty script bodies identify the missing expression", () => {
  const { error } = executeWithLogs({ script: "%dw 2.0\noutput application/json\n---\n" });
  assert.match(error, /empty expression/);
  assert.match(error, /4\| \n\s*\^\nTrace:[\s\S]*line: 4, column: 1/);
});

test("payload decoding errors do not claim a script execution location", () => {
  const { error } = executeWithLogs({ script: "payload", payload: "{", payload_format: "application/json" });
  assert.match(error, /Invalid JSON/);
  assert.doesNotMatch(error, /Trace:/);
});

test("comments do not hide source locations", () => {
  const script = "%dw 2.0\noutput application/json\n---\n// d appears here too\nd";
  const { error } = executeWithLogs({ script });
  assert.match(error, /5\| d[\s\S]*line: 5, column: 1/);
  const unicode = executeWithLogs({ script: '%dw 2.0\noutput application/json\n---\n/* 🍕 */\nd' }).error;
  assert.match(unicode, /5\| d[\s\S]*line: 5, column: 1/);
});

test("repeated expressions point to the one that actually failed", () => {
  const script = '%dw 2.0\noutput application/json\n---\n{\n  handled: try(() -> fail("boom")),\n  failed: fail("boom")\n}';
  const { error } = executeWithLogs({ script });
  assert.match(error, /boom/);
  assert.match(error, /6\|\s+failed: fail\("boom"\)/);
  assert.match(error, /line: 6, column: 11/);
  assert.doesNotMatch(error, /line: 5/);
});
