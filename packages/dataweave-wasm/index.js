"use strict";

const bindings = require("./dist/dwpy_wasm.js");

const parseResult = value => JSON.parse(value);
const encodeRequest = request => typeof request === "string" ? request : JSON.stringify(request);

const analyze = request =>
  parseResult(bindings.analyze_dataweave_request(encodeRequest(request)));

const execute = request =>
  parseResult(bindings.run_dataweave_request(encodeRequest(request)));

module.exports = {analyze, execute};
