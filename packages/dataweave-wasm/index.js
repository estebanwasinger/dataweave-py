"use strict";

const bindings = require("./dist/dwpy_wasm.js");

const parseResult = value => JSON.parse(value);

const analyze = request =>
  parseResult(bindings.analyze_dataweave_request(JSON.stringify(request)));

const execute = request =>
  parseResult(bindings.run_dataweave_request(JSON.stringify(request)));

module.exports = {analyze, execute};
