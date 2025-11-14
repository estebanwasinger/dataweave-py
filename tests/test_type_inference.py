from dwpy.type_inference import infer_script_type
from dwpy.typesystem import STRING, ArrayType, ObjectType


def test_string_concatenation_infers_string():
    inferred = infer_script_type("'hello ' ++ 'world'")
    assert inferred == STRING


def test_object_literal_shape():
    inferred = infer_script_type("{ hola: 'world' }")
    assert isinstance(inferred, ObjectType)
    assert inferred.field_dict()["hola"] == STRING
    assert not inferred.open


def test_array_literal_inference():
    inferred = infer_script_type("[1, 2, 3]")
    assert isinstance(inferred, ArrayType)
    assert inferred.element == infer_script_type("1")


def test_dynamic_object_key_results_in_open_object():
    inferred = infer_script_type('{ "$(payload.name)": "" }')
    assert isinstance(inferred, ObjectType)
    assert inferred.open
