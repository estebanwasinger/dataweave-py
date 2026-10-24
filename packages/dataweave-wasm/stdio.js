"use strict";

const readline = require("readline");
const { analyze, execute, executeWithLogs } = require("./index.js");

const input = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

input.on("line", (line) => {
  if (!line.trim()) return;

  try {
    const request = JSON.parse(line);
    if (request.operation !== "analyze" && request.capture_logs === true) {
      const execution = executeWithLogs(line);
      process.stdout.write(JSON.stringify({
        ok: execution.error === undefined,
        ...execution,
      }) + "\n");
      return;
    }
    const handler = request.operation === "analyze" ? analyze : execute;
    process.stdout.write(JSON.stringify({ ok: true, result: handler(line) }) + "\n");
  } catch (error) {
    process.stdout.write(
      JSON.stringify({
        ok: false,
        error: error instanceof Error ? error.message : String(error),
      }) + "\n",
    );
  }
});
