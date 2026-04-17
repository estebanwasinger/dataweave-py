import pytest

from dwpy.type_inference import infer_script_type
from dwpy.typesystem import (
    ANY,
    BOOLEAN,
    NULL,
    NUMBER,
    STRING,
    ArrayType,
    FunctionType,
    LiteralType,
    ObjectType,
    UnionType,
    union_types,
    array_type,
    object_type,
)


def test_string_concatenation_infers_string():
    inferred = infer_script_type("'hello ' ++ 'world'")
    assert inferred == STRING


def test_object_literal_shape():
    inferred = infer_script_type("{ hola: 'world' }")
    assert isinstance(inferred, ObjectType)
    assert inferred.field_dict()["hola"][0] == STRING
    assert not inferred.open


def test_array_literal_inference():
    inferred = infer_script_type("[1, 2, 3]")
    assert isinstance(inferred, ArrayType)
    assert inferred.element == infer_script_type("1")


def test_dynamic_object_key_results_in_open_object():
    inferred = infer_script_type('{ "$(payload.name)": "" }')
    assert isinstance(inferred, ObjectType)
    assert inferred.open


def test_type_definition_and_coercion_attaches_shape():
    script = """
    %dw 2.0
    type User = {
        name: String,
        age?: Number,
        emails*: String
    }
    ---
    payload as User
    """
    inferred = infer_script_type(script)
    assert isinstance(inferred, ObjectType)
    # optional field becomes nullable
    expected = object_type(
        {
            "name": STRING,
            "age": (union_types(NUMBER, NULL), True, False),
            "emails": (STRING, False, True),
        },
        is_open_flag=True,
    )
    assert inferred.describe() == expected.describe()


def test_closed_object_type_definition():
    script = """
    %dw 2.0
    type Strict = {| name: String, age: Number |}
    ---
    payload as Strict
    """
    inferred = infer_script_type(script)
    assert isinstance(inferred, ObjectType)
    assert not inferred.is_open
    assert inferred.field_dict()["name"][0] == STRING
    assert inferred.field_dict()["age"][0] == NUMBER


def test_union_and_literal_types():
    script = """
    %dw 2.0
    type Code = 200 | 404
    ---
    if (true) 200 else 404 as Code
    """
    inferred = infer_script_type(script)
    assert isinstance(inferred, UnionType)
    # Literal union retains literal members
    literals = {opt.value for opt in inferred.options if isinstance(opt, LiteralType)}
    assert literals == {200, 404}


def test_intersection_merges_object_shapes():
    script = """
    %dw 2.0
    type A = { a: String }
    type B = { b: Number }
    ---
    {} as A & B
    """
    inferred = infer_script_type(script)
    assert isinstance(inferred, ObjectType)
    assert inferred.field_dict()["a"][0] == STRING
    assert inferred.field_dict()["b"][0] == NUMBER


def test_function_type_with_named_params():
    script = """
    %dw 2.0
    type Fn = (s: String, n: Number) -> Boolean
    ---
    ((s, n) -> true) as Fn
    """
    inferred = infer_script_type(script)
    assert isinstance(inferred, FunctionType)
    assert inferred.parameter_types == [STRING, NUMBER]
    assert inferred.return_type == BOOLEAN


def test_lambda_type_inference_defaults_to_any_params():
    inferred = infer_script_type("(x) -> x")
    assert isinstance(inferred, FunctionType)
    assert inferred.parameter_types == [ANY]
    assert inferred.return_type == ANY


def test_default_op_unions_branches():
    inferred = infer_script_type("null default 1")
    assert isinstance(inferred, UnionType)
    assert set(type(opt) for opt in inferred.options) == {type(NULL), type(NUMBER)}


def test_match_expression_unions_cases():
    script = """
    match payload {
      case true -> 1
      case false -> 0
    }
    """
    inferred = infer_script_type(script)
    assert isinstance(inferred, UnionType)
    option_types = {opt.describe() for opt in inferred.options}
    assert "Number" in option_types


def test_payload_projection_preserves_field_type():
    payload_type = object_type({"flag": (BOOLEAN, False, False)}, is_open_flag=False)
    inferred = infer_script_type("payload.flag", payload_type=payload_type)
    assert inferred == BOOLEAN


def test_array_payload_projection_infers_array_of_field_type():
    payload_type = array_type(
        object_type({"message": (STRING, False, False)}, is_open_flag=False)
    )
    inferred = infer_script_type("payload.message", payload_type=payload_type)
    assert isinstance(inferred, ArrayType)
    assert inferred.element == STRING


@pytest.mark.parametrize(
    "script,expected",
    [
        ("[1, 2, 3] ++ [4]", ArrayType(NUMBER)),
        ("'a' ++ 'b'", STRING),
        ("[1] ++ ['a']", ArrayType(union_types(NUMBER, STRING))),
    ],
)
def test_concat_shapes(script, expected):
    inferred = infer_script_type(script)
    assert inferred.describe() == expected.describe()


def test_infer_payload_object_plus_literal_object():
    payload = {"some": "value"}
    inferred = infer_script_type(
        """
        payload ++ { "other": "value" }
        """,
        payload_type=payload,
    )
    assert isinstance(inferred, ObjectType)
    fields = inferred.field_dict()
    assert "some" in fields and fields["some"][0] == STRING
    assert "other" in fields and fields["other"][0] == STRING


def test_object_minus_removes_key_when_literal():
    payload = {"some": "value", "value": "v"}
    inferred = infer_script_type(
        """
        %dw 2.0
        ---
        payload - "some"
        """,
        payload_type=payload,
    )
    assert isinstance(inferred, ObjectType)
    fields = inferred.field_dict()
    assert "some" not in fields
    assert "value" in fields


def test_range_map_results_in_array_of_numbers():
    payload = {"some": "value", "value": "value"}
    inferred = infer_script_type(
        """
        %dw 2.0
        ---
        { val : (1 to 10 map $) }
        """,
        payload_type=payload,
    )
    assert isinstance(inferred, ObjectType)
    fields = inferred.field_dict()
    assert isinstance(fields["val"][0], ArrayType)
    assert fields["val"][0].element == NUMBER


def test_range_map_with_coercion_results_in_array_of_strings():
    inferred = infer_script_type(
        """
        %dw 2.0
        ---
        { val : (1 to 10 map ($ as String)) }
        """
    )
    assert isinstance(inferred, ObjectType)
    val_field = inferred.field_dict()["val"][0]
    assert isinstance(val_field, ArrayType)
    assert val_field.element == STRING


def test_range_filter_results_in_array_of_numbers():
    inferred = infer_script_type(
        """
        %dw 2.0
        ---
        { val : 1 to 10 filter $ < 5 }
        """
    )
    assert isinstance(inferred, ObjectType)
    val_field = inferred.field_dict()["val"][0]
    assert isinstance(val_field, ArrayType)
    assert val_field.element == NUMBER


def test_unary_not_infers_boolean():
    inferred = infer_script_type("!payload.flag")
    assert inferred == BOOLEAN


def test_range_selector_over_array_preserves_array_type():
    inferred = infer_script_type(
        """
        %dw 2.0
        ---
        (1 to 10)[-1 to 0]
        """
    )
    assert isinstance(inferred, ArrayType)
    assert inferred.element == NUMBER


def test_group_by_preserves_value_shape():
    inferred = infer_script_type(
        """
        %dw 2.0
        fun concat(a, b) = a ++ b
        ---
        { val : [
            {
                "age" : 5,
                "name" : "Esteban"
            }
        ] groupBy ((item, index) -> item.age)}
        """
    )
    assert isinstance(inferred, ObjectType)
    val_field = inferred.field_dict()["val"][0]
    assert isinstance(val_field, ObjectType)
    grouped_array_type = val_field.field_dict().get("__grouped__", (None, False, False))[0]
    assert isinstance(grouped_array_type, ArrayType)
    elem = grouped_array_type.element
    assert isinstance(elem, ObjectType)
    assert elem.field_dict().get("age")[0] == NUMBER
