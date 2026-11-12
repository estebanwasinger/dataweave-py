import pytest

from dwpy.parser import ParseError
from dwpy.runtime import DataWeaveEvaluationError, DataWeaveRuntime
from dwpy.type_inference import infer_script_type
from dwpy.typesystem import ObjectType, STRING


@pytest.mark.parametrize("backend", ["python", "rust"])
@pytest.mark.parametrize(
    "expression, expected",
    [
        ('({"x": "y"} ++ ("y"): "z")', {"x": "y", "y": "z"}),
        ('({"x": "y"} ++ (("y"): "z"))', {"x": "y", "y": "z"}),
        ('{outer: {x: 1} ++ (upper("y")): 2 + 3}', {"outer": {"x": 1, "Y": 5}}),
        ('({x: 1} ++ ("y" ++ ":z"): "value")', {"x": 1, "y:z": "value"}),
        ('({x: 1} ++ // : comment\n ("y"): 2)', {"x": 1, "y": 2}),
        ('{x: 1} ++ y: "a" ++ "b"', {"x": 1, "y": "ab"}),
        ('{x: 1, y: 2} -- y: 0', {"x": 1}),
        ('[a: 1, ("b"): 2]', [{"a": 1}, {"b": 2}]),
        ('("y"): "z"', {"y": "z"}),
    ],
)
def test_single_entry_object_expressions(monkeypatch, backend, expression, expected):
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    result = DataWeaveRuntime(backend=backend).execute(
        expression, {}, render_output=False
    )
    assert result == expected


@pytest.mark.parametrize("backend", ["python", "rust"])
def test_single_entry_object_requires_a_value(monkeypatch, backend):
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    with pytest.raises((ParseError, DataWeaveEvaluationError)):
        DataWeaveRuntime(backend=backend).execute('{x: 1} ++ ("y"):', {})


def test_single_entry_object_type_inference():
    inferred = infer_script_type('{"x": "y"} ++ ("y"): "z"')
    assert isinstance(inferred, ObjectType)
    assert inferred.field_dict()["x"][0] == STRING
    assert inferred.field_dict()["y"][0] == STRING
