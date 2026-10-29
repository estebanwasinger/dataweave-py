import json

import pytest
import yaml

from dwpy.runtime import DataWeaveRuntime


@pytest.mark.parametrize('backend', ['python', 'rust', 'wasm'])
def test_output_mime_override(backend):
    runtime = DataWeaveRuntime(backend=backend)
    script = '%dw 2.0\noutput application/json indent=true\n---\n{message: payload.name}'
    assert yaml.safe_load(runtime.execute(script, {'name': 'Ada'}, output_mime_type='application/yaml')) == {'message': 'Ada'}
    assert runtime.execute(script, {'name': 'Ada'}, output_mime_type='application/json') == '{"message":"Ada"}'
    assert runtime.execute(script, {'name': 'Ada'}, output_mime_type='application/python') == {'message': 'Ada'}
    assert runtime.execute(script, {'name': 'Ada'}, output_mime_type='application/yaml', render_output=False) == {'message': 'Ada'}
    assert json.loads(runtime.execute('payload', {'name': 'Ada'}, output_mime_type='application/json')) == {'name': 'Ada'}
    with pytest.raises(Exception, match='Unsupported output format'):
        runtime.execute(script, {}, output_mime_type='application/unknown')
