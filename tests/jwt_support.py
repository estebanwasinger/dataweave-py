"""Independent JWT signature checks using public test key material only."""

import base64
import hashlib
import hmac
import json
from pathlib import Path

FIXTURES = Path(__file__).parent / "fixtures" / "jwt"
RSA_PRIVATE_KEY = (FIXTURES / "test-only-rsa-pkcs8.pem").read_text()


def decode_segment(value: str) -> bytes:
    return base64.urlsafe_b64decode(value + "=" * (-len(value) % 4))


def verify_rsa_token(token: str, algorithm: str, header: dict, payload: dict) -> bool:
    """Verify RSASSA-PKCS1-v1_5 without calling the implementation under test."""
    try:
        first, second, third = token.split(".")
        if any("=" in segment for segment in (first, second, third)):
            return False
        if json.loads(decode_segment(first)) != {**header, "alg": algorithm, "typ": "JWT"}:
            return False
        if json.loads(decode_segment(second)) != payload:
            return False
        public_key = json.loads((FIXTURES / "public-key.json").read_text())
        modulus = int(public_key["n"], 16)
        signature = decode_segment(third)
        length = (modulus.bit_length() + 7) // 8
        if len(signature) != length or int.from_bytes(signature, "big") >= modulus:
            return False
        encoded = pow(int.from_bytes(signature, "big"), public_key["e"], modulus).to_bytes(length, "big")
        prefix = {
            "RS256": "3031300d060960864801650304020105000420",
            "RS384": "3041300d060960864801650304020205000430",
            "RS512": "3051300d060960864801650304020305000440",
        }[algorithm]
        digest = hashlib.new("sha" + algorithm[2:], f"{first}.{second}".encode()).digest()
        digest_info = bytes.fromhex(prefix) + digest
        expected = b"\x00\x01" + b"\xff" * (length - len(digest_info) - 3) + b"\x00" + digest_info
        return hmac.compare_digest(encoded, expected)
    except (ValueError, KeyError, TypeError):
        return False
