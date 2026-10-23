const test = require("node:test");
const assert = require("node:assert/strict");
const { execute } = require("../index.js");

const payload = '{"id":1}\nnot-json\n{"id":2}\n';

for (const format of ["ndjson", "application/x-ndjson", "application/x-ldjson"]) {
  test(`${format} payload reports a data error and supports skipInvalid`, () => {
    const request = { script: "payload", payload, payload_format: format, render_output: false };
    assert.throws(
      () => execute(request),
      /^Invalid JSON value: Invalid NDJSON record on line 2:/,
    );
    assert.deepEqual(
      execute({ ...request, payload_format_options: { skipInvalid: true } }),
      [{ id: 1 }, { id: 2 }],
    );
  });

  test(`${format} read reports the malformed record's line`, () => {
    assert.throws(
      () => execute({ script: `read(payload, "${format}")`, payload }),
      /^Invalid JSON value: Invalid NDJSON record on line 2:/,
    );
  });
}
