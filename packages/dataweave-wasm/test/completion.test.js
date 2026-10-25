const test = require("node:test");
const assert = require("node:assert/strict");
const { complete } = require("..");

test("evaluator operators such as update are suggested without function-call syntax", () => {
  const script = "%dw 2.0\n---\n{numA: 2} up";
  const items = complete({ script, line: 2, column: 12 }).items.filter(i => i.label === "update");
  assert.equal(items.length, 1);
  assert.equal(items[0].kind, "keyword");
  assert.equal(items[0].insertText, "update");
  assert.equal(items[0].insertTextFormat, "plain");
});

test("source-only completions expose typed overloads through the Node API", () => {
  const script = "%dw 2.0\nfun multiply(n, factor: Number) = n * factor\nfun multiply(n) = n\n---\nm";
  const { items } = complete({ script, line: 4, column: 1 });
  const locals = items.filter(i => i.label === "multiply");
  assert.equal(locals.length, 2);
  assert.deepEqual(new Set(locals.map(i => i.insertText)), new Set(["multiply(${1:n})", "multiply(${1:n}, ${2:factor})"]));
  assert.deepEqual(complete(JSON.stringify({ script, line: 4, column: 1 })), { items });
});

test("completion input is validated and declarations are never executed", () => {
  assert.throws(() => complete({ script: 42, line: 0, column: 0 }));
  assert.throws(() => complete({ script: "m", line: -1, column: 0 }));
  const { items } = complete({ script: '%dw 2.0\nvar message = fail("do not run")\n---\nm', line: 3, column: 1 });
  assert.equal(items[0].label, "message");
});

test("completion uses the same structured input readers as execution", () => {
  for (const [payload, payload_format, script] of [
    ["name,city\nAda,London", "application/csv", "payload[0]."],
    ["name: Ada\ncity: London", "application/yaml", "payload."],
    ["<row><name>Ada</name><city>London</city></row>", "application/xml", "payload.row."],
  ]) {
    const { items } = complete({ script, line: 0, column: script.length, payload, payload_format });
    assert.deepEqual(items.map(i => i.label), ["name", "city"], payload_format);
    assert.ok(items.every(i => i.kind === "field"));
  }
});

test("implicit and named callback selectors use the current input element", () => {
  const payload = [{ name: "Esteban", lastName: "Wasinger", children: [{ nickname: "E" }] }, { email: "e@example.test" }];
  for (const script of ["payload map $.", "payload filter $.", "payload map ((item) -> item."]) {
    const items = complete({ script, line: 0, column: script.length, payload }).items;
    assert.deepEqual(items.map(i => i.label), ["name", "lastName", "children", "email"]);
    assert.ok(items.every(i => i.kind === "field"));
  }
  const script = "payload map ($.children map $.ni";
  assert.deepEqual(complete({ script, line: 0, column: script.length, payload }).items.map(i => i.label), ["nickname"]);
  const csv = "payload map $.";
  assert.deepEqual(complete({ script: csv, line: 0, column: csv.length, payload: "name,lastName\nEsteban,Wasinger", payload_format: "application/csv" }).items.map(i => i.label), ["name", "lastName"]);
  for (const script of ["$.", "payload map $$.", "[payload map $.name, $.]", "fail(\"never\") map $."]) {
    const column = script.lastIndexOf(".") + 1;
    assert.deepEqual(complete({ script, line: 0, column, payload }).items, []);
  }
  assert.deepEqual(complete({ script: csv, line: 0, column: csv.length, payload: [] }).items, []);
  assert.deepEqual(complete({ script: csv, line: 0, column: csv.length, payload: [{ addedLater: true }] }).items.map(i => i.label), ["addedLater"]);
});

test("invalid data does not prevent source completions", () => {
  const request = { script: "pa", line: 0, column: 2, payload: "{invalid", payload_format: "application/json" };
  assert.equal(complete(request).items[0].label, "payload");
  assert.deepEqual(complete({ ...request, script: "payload.", column: 8 }).items, []);
});

test("all active-engine functions are offered through the same dynamic catalog", () => {
  const { getFunctionCatalog } = require("..");
  const catalog = getFunctionCatalog();
  const items = complete({ script: "", line: 0, column: 0 }).items;
  for (const entry of catalog) {
    assert.ok(items.some(i => i.label === entry.name && i.kind === "function"), entry.name);
    assert.deepEqual(new Set(entry.signatures.map(s => s.parameters.length)), new Set(entry.arities));
  }
});

test("source and imported module function edits update the next completion request", () => {
  const source = "%dw 2.0\nimport custom::Tools as Tools\n---\nTools::new";
  const request = { script: source, line: 3, column: 10, module_sources: {
    "custom::Tools": "%dw 2.0\nfun oldFunction(n) = n\n---\nnull",
  }};
  assert.ok(!complete(request).items.some(i => i.label === "Tools::newFunction"));
  request.module_sources["custom::Tools"] = "%dw 2.0\nfun newFunction(first, second) = first\n---\nnull";
  assert.equal(complete(request).items.find(i => i.label === "Tools::newFunction").insertText,
    "Tools::newFunction(${1:first}, ${2:second})");
});
