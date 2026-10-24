from typing import get_args, get_origin

import pytest
from pydantic import BaseModel, ValidationError

from dwpy.pydantic_export import pydantic_model_from_script, python_type_from_dw_type
from dwpy.typesystem import STRING, object_type


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
    instance = Model(hi="there", other="value")
    data = instance.model_dump()
    assert "hi" in data and "other" in data


def test_array_script_produces_list_annotation():
    script = "[1, 2, 3]"
    py_type = pydantic_model_from_script(script, model_name="Numbers")
    assert get_origin(py_type) is list or get_origin(py_type) is list  # typing.List
    (element_type,) = get_args(py_type)
    assert element_type is float


def test_object_type_preserves_optional_repeated_fields():
    output_type = object_type({"name": STRING, "tags": (STRING, True, True)}, open=False)
    model = python_type_from_dw_type(output_type)

    assert model(name="Ada").model_dump() == {"name": "Ada", "tags": None}
    assert model(name="Ada", tags=["one"]).model_dump()["tags"] == ["one"]
    with pytest.raises(ValidationError):
        model(name="Ada", tags="one")
