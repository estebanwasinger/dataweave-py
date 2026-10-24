import hashlib
import hmac
import json
import os

import pytest
from jwt_support import FIXTURES, RSA_PRIVATE_KEY, decode_segment, verify_rsa_token

from dwpy.runtime import DataWeaveEvaluationError, DataWeaveRuntime

pytestmark = pytest.mark.skipif(
    os.environ.get("DWPY_TEST_BACKEND") == "python",
    reason="JWT signing is implemented by the native Rust and WASM engines",
)


def execute(script, *, key=None):
    return DataWeaveRuntime(backend="rust").execute(
        script, {}, vars={"key": key or RSA_PRIVATE_KEY}, render_output=False
    )


@pytest.mark.parametrize("bits", [256, 384, 512])
def test_hmac_jwt_signatures_and_nested_payload(bits):
    key = "é" * (bits // 16)
    token = execute(f'''%dw 2.0
import * from dw::util::jwt::HMAC
output application/json
---
{{tokens: [JWT({{kid: "example", alg: "none", typ: "invalid"}}, {{sub: "café", roles: ["reader"], nested: {{active: true}}}}, vars.key, "HmacSHA{bits}")]}}.tokens[0]
''', key=key)
    first, second, third = token.split(".")
    assert json.loads(decode_segment(first)) == {"kid": "example", "alg": f"HS{bits}", "typ": "JWT"}
    assert json.loads(decode_segment(second)) == {"sub": "café", "roles": ["reader"], "nested": {"active": True}}
    expected = hmac.new(key.encode(), f"{first}.{second}".encode(), getattr(hashlib, f"sha{bits}")).digest()
    assert hmac.compare_digest(decode_segment(third), expected)
    assert "=" not in token


@pytest.mark.parametrize("imports, call", [
    ("import * from dw::util::jwt::HMAC", "JWT"),
    ("import JWT from dw::util::jwt::HMAC", "JWT"),
    ("import JWT as sign from dw::util::jwt::HMAC", "sign"),
    ("import dw::util::jwt::HMAC", "HMAC::JWT"),
    ("import dw::util::jwt::HMAC as Tokens", "Tokens::JWT"),
    ("", "dw::util::jwt::HMAC::JWT"),
])
def test_jwt_import_forms_and_default_overload(imports, call):
    token = execute(f'%dw 2.0\n{imports}\n---\n{call}({{sub: "test"}}, vars.key)', key="k" * 32)
    assert json.loads(decode_segment(token.split(".")[0])) == {"alg": "HS256", "typ": "JWT"}


@pytest.mark.parametrize("bits", [256, 384, 512])
@pytest.mark.parametrize("key_format", ["pkcs1", "pkcs8"])
def test_rsa_jwt_signatures(bits, key_format):
    key = (FIXTURES / f"test-only-rsa-{key_format}.pem").read_text()
    token = execute(f'''%dw 2.0
import JWT as sign from dw::util::jwt::RSA
output application/json
// This is a native input value supplied through vars, as in the docs.
input key application/json
---
sign({{kid: "example"}}, {{sub: "test"}}, key, "Sha{bits}withRSA")
''', key=key)
    assert verify_rsa_token(token, f"RS{bits}", {"kid": "example"}, {"sub": "test"})
    first, second, signature = token.split(".")
    tampered = f"{first}.{second}.{'A' if signature[0] != 'A' else 'B'}{signature[1:]}"
    assert not verify_rsa_token(tampered, f"RS{bits}", {"kid": "example"}, {"sub": "test"})


def test_rsa_default_overload_and_distinct_module_bindings():
    tokens = execute('''%dw 2.0
import JWT as hs from dw::util::jwt::HMAC
import JWT as rs from dw::util::jwt::RSA
---
[hs({sub: "test"}, "01234567890123456789012345678901"), rs({sub: "test"}, vars.key)]
''')
    assert json.loads(decode_segment(tokens[0].split(".")[0]))["alg"] == "HS256"
    assert verify_rsa_token(tokens[1], "RS256", {}, {"sub": "test"})


@pytest.mark.parametrize("module, arguments, key, message", [
    ("HMAC", '{sub: "test"}, vars.key', "short", "at least 256 bits"),
    ("HMAC", '{}, {}, vars.key, "HmacSHA512"', "k" * 63, "at least 512 bits"),
    ("HMAC", '{}, {}, vars.key, "none"', "k" * 32, "unsupported signing algorithm"),
    ("HMAC", '[], vars.key', "k" * 32, "payload must be an Object"),
    ("HMAC", '[], {}, vars.key', "k" * 32, "header must be an Object"),
    ("HMAC", '{}, {}, 123', "unused", "signingKey must be a String"),
    ("HMAC", '{}, {}, vars.key, 123', "k" * 32, "algorithm must be a String"),
    ("HMAC", '{}', "unused", "expected"),
    ("RSA", '{}, vars.key', "invalid test key", "unencrypted PKCS#1 or PKCS#8"),
    ("RSA", '{}, vars.key', (FIXTURES / "test-only-rsa-public.pem").read_text(), "private key"),
    ("RSA", '{}, vars.key', (FIXTURES / "test-only-rsa-1024.pem").read_text(), "at least 2048 bits"),
    ("RSA", '{}, {}, vars.key, "HmacSHA256"', RSA_PRIVATE_KEY, "unsupported signing algorithm"),
])
def test_jwt_rejects_invalid_arguments_without_exposing_key(module, arguments, key, message):
    with pytest.raises(DataWeaveEvaluationError, match=message) as error:
        execute(f'%dw 2.0\nimport * from dw::util::jwt::{module}\n---\nJWT({arguments})', key=key)
    assert key not in str(error.value)


def test_named_input_requires_a_supplied_value():
    with pytest.raises(DataWeaveEvaluationError, match="Missing named input 'missing'"):
        execute('%dw 2.0\ninput missing application/json\n---\nmissing')


def test_jwt_type_is_string():
    runtime = DataWeaveRuntime(backend="rust")
    assert runtime._rust_runtime is not None
    assert runtime._rust_runtime.infer_type_descriptor('dw::util::jwt::HMAC::JWT({}, vars.key)')["kind"] == "String"
