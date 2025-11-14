from typing import get_origin, get_args

from pydantic import BaseModel, ValidationError

from dwpy.pydantic_export import pydantic_model_from_script


def test_string_script_produces_str_type():
    script = "'hello ' ++ 'world'"
    py_type = pydantic_model_from_script(script)
    assert py_type is str


def test_object_script_produces_model():
    script = "{ hola: 'world', count: 1 }"
    Model = pydantic_model_from_script(script, model_name="Greeting")
    assert issubclass(Model, BaseModel)
    instance = Model(hola="hola", count=42)
    assert instance.model_dump() == {"hola": "hola", "count": 42}
    try:
        Model(other="value")
    except ValidationError:
        pass
    else:
        assert False, "extra fields should be forbidden"


def test_dynamic_key_results_in_extra_allow_model():
    script = '{ "$(payload.name)": "" }'
    Model = pydantic_model_from_script(script, model_name="Dynamic")
    assert issubclass(Model, BaseModel)
    instance = Model(**{"hi": "there", "other": "value"})
    data = instance.model_dump()
    assert "hi" in data and "other" in data


def test_array_script_produces_list_annotation():
    script = "[1, 2, 3]"
    py_type = pydantic_model_from_script(script, model_name="Numbers")
    assert get_origin(py_type) is list or get_origin(py_type) is list  # typing.List
    (element_type,) = get_args(py_type)
    assert element_type is float
