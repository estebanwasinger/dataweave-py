const test = require('node:test');
const assert = require('node:assert/strict');
const {execute, executeWithLogs, analyze} = require('../index.js');

const bindings = {record: {rent: 5}, tenant: {name: 'Ada'}, numbers: [1, 2], count: 3, properties: 4};

test('supplies bindings to execution, analysis, and logging', () => {
  const script = '{rent: record.rent, name: tenant.name, count: count, first: numbers[0], existing: vars.x, properties: properties}';
  const request = {script, bindings, vars: {x: 9}, payload: {}, render_output: false};
  assert.deepEqual(execute(request), {rent: 5, name: 'Ada', count: 3, first: 1, existing: 9, properties: 4});
  assert.equal(executeWithLogs({...request, script: 'log(record.rent)'}).result, 5);
  const analysis = analyze({expression: 'record.rent', bindings});
  assert.equal(analysis.inferredType.kind, 'Number');
  assert.deepEqual(analysis.references, ['record.rent']);
  assert.deepEqual(analysis.unresolvedReferences, []);
  assert.deepEqual(analyze({expression: 'count', bindings}).unresolvedReferences, []);
  assert.deepEqual(analyze({expression: 'numbers[0]', bindings}).unresolvedReferences, []);
  assert.equal(analyze({expression: 'record.rent + count', bindings}).inferredType.kind, 'Number');
  assert.deepEqual(analyze({expression: 'record.missing', bindings}).unresolvedReferences, ['record.missing']);
  assert.deepEqual(analyze({expression: 'unknown.field', bindings}).unresolvedReferences, ['unknown.field']);
  assert.deepEqual(analyze({expression: 'record.rent', bindings: {}, payload: {record: {rent: 5}}}).unresolvedReferences, ['record.rent']);
  assert.throws(() => execute({script: 'unknown.field', bindings, render_output: false}));
});

test('accepts bindings on typed transport requests', () => {
  const request = {bindings: {record: {rent: 5}}, typed_transport: true};
  assert.equal(execute({...request, script: 'record.rent', render_output: false}), 5);
  assert.equal(analyze({...request, expression: 'record.rent'}).inferredType.kind, 'Number');
});

test('keeps lexical reference analysis with bindings', () => {
  const context = {bindings: {joinBy: 'yes', record: {rent: 5}}};
  const result = analyze({...context, expression: '[joinBy, record.rent]'});
  assert.deepEqual(result.references, ['joinBy', 'record.rent']);
  assert.deepEqual(result.unresolvedReferences, []);
  assert.throws(() => analyze({...context, expression: 'record.rent +'}), /missing expression/);
});

test('validates binding names and supports header shadowing', () => {
  for (const name of ['payload', 'vars', 'attributes', 'bad-name', '__dwpy_properties', 'true']) {
    assert.throws(() => execute({script: '1', bindings: {[name]: 1}}), /binding name/);
    assert.throws(() => analyze({expression: '1', bindings: {[name]: 1}}), /binding name/);
  }
  assert.throws(() => execute({script: '1', bindings: []}), /bindings must be a JSON object/);
  assert.throws(() => analyze({expression: '1', bindings: []}), /bindings must be a JSON object/);
  assert.equal(execute({script: 'var count = 8\n---\ncount', bindings, render_output: false}), 8);
  assert.equal(execute({script: 'var a = record.rent\n---\na + 1', bindings, render_output: false}), 6);
  assert.equal(analyze({expression: 'var count = "a"\n---\ncount', bindings}).inferredType.kind, 'Any');
});
