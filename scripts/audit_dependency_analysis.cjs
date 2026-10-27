#!/usr/bin/env node
"use strict";

// Audit saved execution requests without changing the runtime or test suites.
// Cases with an expected result are also executed, establishing that an
// analysis failure concerns a working expression rather than invalid syntax.
const fs = require("node:fs");
const path = require("node:path");
const { isDeepStrictEqual } = require("node:util");
const { analyze, execute } = require("../packages/dataweave-wasm");

let input = path.join(__dirname, "../tests/fixtures/dependency-analysis/gaps.json");
let output;
let suppliedInput = false;
const args = process.argv.slice(2);
for (let index = 0; index < args.length; index += 1) {
  if (args[index] === "--output") {
    output = args[++index];
    if (!output || output.startsWith("--")) throw new Error("--output requires a path");
  } else if (args[index].startsWith("--") || suppliedInput) {
    throw new Error("Usage: node scripts/audit_dependency_analysis.cjs [requests.json|requests.jsonl] [--output report.json]");
  } else {
    input = args[index];
    suppliedInput = true;
  }
}
const source = fs.readFileSync(input, "utf8").trim();
const records = source.startsWith("[") ? JSON.parse(source) : source.split("\n").filter(Boolean).map(JSON.parse);
const unique = new Map();
for (const record of records) {
  if (record.success === false) continue;
  const request = record.request ?? record;
  if (typeof request.expression !== "string") continue;
  const key = JSON.stringify([
    request.expression, request.payload, request.vars, request.attributes,
    request.payload_format, request.typed_transport,
    record.expectedReferences, record.expectedUnresolvedReferences,
  ]);
  if (!unique.has(key)) unique.set(key, { ...record, request });
}

const results = [];
for (const record of unique.values()) {
  const { request } = record;
  const result = {
    id: record.id,
    sourceTest: record.sourceTest ?? record.test,
    origin: record.origin,
    expression: request.expression,
    failures: [],
  };
  if (Object.hasOwn(record, "expectedResult")) {
    try {
      result.executionResult = execute({ ...request, script: request.expression, render_output: false });
      if (!isDeepStrictEqual(result.executionResult, record.expectedResult)) {
        result.failures.push("execution result mismatch");
      }
    } catch (error) {
      result.executionError = String(error);
      result.failures.push("execution failed");
    }
  }
  try {
    result.analysis = analyze(request);
    for (const [expectation, actual] of [
      ["expectedReferences", "references"],
      ["expectedUnresolvedReferences", "unresolvedReferences"],
    ]) {
      if (Object.hasOwn(record, expectation) && !isDeepStrictEqual(result.analysis[actual], record[expectation])) {
        result.failures.push(`${actual} mismatch`);
      }
    }
  } catch (error) {
    result.analysisError = String(error);
    result.failures.push("analysis failed");
  }
  results.push(result);
}
const summary = {
  inputRecords: records.length,
  checkedRequests: results.length,
  distinctExpressions: new Set(results.map(r => r.expression)).size,
  executionChecks: results.filter(r => Object.hasOwn(r, "executionResult") || r.executionError).length,
  executionFailures: results.filter(r => r.failures.some(f => f.startsWith("execution"))).length,
  analysisErrors: results.filter(r => r.analysisError).length,
  dependencyMismatches: results.filter(r => r.failures.some(f => f.endsWith("mismatch") && !f.startsWith("execution"))).length,
  unresolvedCandidates: results.filter(r => r.analysis?.unresolvedReferences.length).length,
  casesWithFailures: results.filter(r => r.failures.length).length,
};
if (output) {
  fs.writeFileSync(output, `${JSON.stringify({ summary, results }, null, 2)}\n`);
}
console.log(JSON.stringify(summary, null, 2));
// Unresolved references alone are candidates, not proof of a defect: optional
// fields, empty arrays, and intentionally handled missing values are valid.
process.exitCode = summary.casesWithFailures ? 1 : 0;
