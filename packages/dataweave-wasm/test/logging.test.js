const test = require("node:test");
const assert = require("node:assert/strict");
const {execute, executeWithLogs} = require("../index.js");

test("injects multiple loggers and keeps the result-only API compatible", () => {
  const first = [], second = [];
  const request = {script: "%dw 2.0\noutput application/json\n---\n1 to 10 map log($)"};
  assert.equal(execute(request, {loggers: [entry => first.push(entry), entry => second.push(entry)]}), "[1,2,3,4,5,6,7,8,9,10]");
  assert.deepEqual(first, second);
  assert.equal(first.length, 10);
  assert.deepEqual(first[0], {level: "warn", prefix: null, message: "1"});
  assert.equal(execute(request), "[1,2,3,4,5,6,7,8,9,10]");
});

test("captures all levels, prefix evaluation, and logs before errors", () => {
  const execution = executeWithLogs({script: '[logDebug("a"), logInfo("b"), logWarn("c"), logError("d"), logWith("ERROR", "label", 5), fail("boom")]'});
  assert.match(execution.error, /boom/);
  assert.deepEqual(execution.logs.map(entry => entry.level), ["debug", "info", "warn", "error", "error"]);
  assert.equal(execution.logs[4].message, "label - 5");
  const entries = [];
  assert.throws(() => execute({script: '[log("before"), fail("boom")]'}, {loggers: [entry => entries.push(entry)]}), /boom/);
  assert.equal(entries[0].message, "before");
  assert.deepEqual(executeWithLogs({script: "42"}).logs, []);
});

test("bounds capture by count and Unicode message size", () => {
  const execution = executeWithLogs({script: "1 to 1005 map log($)"});
  assert.equal(execution.logs.length, 1000);
  assert.equal(execution.result.length, 1005);
  assert.equal(execution.logs_truncated, true);
  const large = executeWithLogs({script: "log(payload)", payload: "🦋".repeat(10000)});
  assert.equal(large.result, "🦋".repeat(10000));
  assert.ok(Buffer.byteLength(large.logs[0].message) < 16400);
  assert.equal(large.logs_truncated, true);
});

test("raw duplicate-bearing request and log values remain intact", () => {
  const execution = executeWithLogs('{"script":"log(payload).*a","payload":{"a":1,"a":2}}');
  assert.deepEqual(execution.result, [1, 2]);
  assert.equal(execution.logs[0].message, '{"a":1,"a":2}');
});

test("logger configuration and callback errors propagate", () => {
  assert.throws(() => execute({script: "42"}, {loggers: [42]}), /loggers/);
  assert.throws(() => execute({script: "log(1)"}, {loggers: [() => {throw new Error("sink failed");}]}), /sink failed/);
});

test("JSON rendering errors and complex objects do not replay logs", () => {
  for (const body of ['[logInfo("before", 1), fail("boom")]', '{first: log(1), (payload)}', '[log(1), (2) if false]']) {
    const execution = executeWithLogs({script: "%dw 2.0\noutput application/json\n---\n" + body, payload: {other: 2}});
    assert.equal(execution.logs.length, 1, body);
    if (body.includes("fail(")) assert.match(execution.error, /boom/);
    else assert.equal(execution.error, undefined);
  }
});
