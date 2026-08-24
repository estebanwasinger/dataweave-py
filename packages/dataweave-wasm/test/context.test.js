const test = require("node:test");
const assert = require("node:assert/strict");
const {execute, analyze} = require("../index.js");

test("executes the full DataWeave context", () => {
  const result = execute({
    script: `%dw 2.0
output application/python
---
{value: vars.myVariable, header: attributes.headers.x, env: p("env"), canonical: Mule::p("env"), dotted: p("app.config.value"), missing: p("missing")}`,
    payload: {},
    vars: {myVariable: 7},
    attributes: {headers: {x: "yes"}},
    properties: {env: "dev", "app.config.value": "flat"},
    render_output: false,
  });
  assert.deepEqual(result, {
    value: 7,
    header: "yes",
    env: "dev",
    canonical: "dev",
    dotted: "flat",
    missing: null,
  });
});

test("rejects non-string property values", () => {
  assert.throws(
    () => execute({script: 'p("env")', properties: {env: 1}}),
    /property 'env' must be a string value/
  );
});

test("analyzes attributes and property functions", () => {
  assert.equal(
    analyze({expression: "attributes.headers.x", attributes: {headers: {x: "yes"}}}).inferredType.kind,
    "String"
  );
  assert.deepEqual(analyze({expression: 'p("env")'}).inferredType, {
    kind: "Union",
    options: [{kind: "String"}, {kind: "Null"}],
  });
});
