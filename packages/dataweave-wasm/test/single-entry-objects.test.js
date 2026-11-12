const test = require('node:test');
const assert = require('node:assert/strict');
const {execute, analyze} = require('../index.js');

test('concatenates an unbraced dynamic-key object', () => {
  const script = '({"x": "y"} ++ ("y"): "z")';
  assert.deepEqual(execute({script, payload: {}, render_output: false}), {x: 'y', y: 'z'});
  const analysis = analyze({expression: script});
  assert.deepEqual(analysis.diagnostics, []);
  assert.equal(analysis.inferredType.kind, 'Object');
  assert.deepEqual(Object.keys(analysis.inferredType.fields).sort(), ['x', 'y']);
});
