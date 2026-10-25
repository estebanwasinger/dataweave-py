"use strict";

const bindings = require("./dist/dwpy_wasm.js");

const parseResult = value => JSON.parse(value);
const encodeRequest = request => typeof request === "string" ? request : JSON.stringify(request);

const getFunctionCatalog = () => parseResult(bindings.get_dataweave_function_catalog());

const complete = request =>
  parseResult(bindings.complete_dataweave_request(encodeRequest(request)));

const analyze = request =>
  parseResult(bindings.analyze_dataweave_request(encodeRequest(request)));

const executeWithLogs = request =>
  parseResult(bindings.run_dataweave_request_with_logs(encodeRequest(request)));

const execute = (request, options) => {
  if (options?.loggers === undefined) {
    return parseResult(bindings.run_dataweave_request(encodeRequest(request)));
  }
  if (!Array.isArray(options.loggers) || options.loggers.some(logger => typeof logger !== "function")) {
    throw new TypeError("loggers must be an array of LogEntry callbacks");
  }
  const execution = executeWithLogs(request);
  for (const entry of execution.logs) {
    for (const logger of options.loggers) logger(entry);
  }
  if (execution.error !== undefined) {
    throw Object.assign(new Error(execution.error), {
      logs: execution.logs,
      logs_truncated: execution.logs_truncated,
    });
  }
  return execution.result;
};

class DebugAdapter {
  constructor() { this.adapter = new bindings.DataWeaveDebugAdapter(); }
  dispatch(request) { return parseResult(this.adapter.dispatch(encodeRequest(request))); }
  poll(fuel = 200) { return parseResult(this.adapter.poll(fuel)); }
  get running() { return this.adapter.is_running(); }
  dispose() { this.adapter.free(); }
}
module.exports = {getFunctionCatalog, complete, analyze, execute, executeWithLogs, DebugAdapter};
