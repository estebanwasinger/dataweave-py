use std::io::{self, Write};

use digest::Digest;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::rand_core::OsRng;
use rsa::traits::PublicKeyParts;
use rsa::{Pkcs1v15Sign, RsaPrivateKey};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::builtins::{base64_encode, binary_value, hmac_bytes};
use crate::DwError;

fn jwt_error(message: &str) -> DwError {
    DwError::InvalidJson(format!("JWT: {message}"))
}

pub(crate) fn sign(module: &str, arguments: &[Value]) -> Result<Value, DwError> {
    let rsa = module == "RSA";
    let (header, payload, key, algorithm) = match arguments {
        [payload, key] => (
            Map::new(),
            payload,
            key,
            if rsa { "Sha256withRSA" } else { "HmacSHA256" },
        ),
        [header, payload, key] => (
            header
                .as_object()
                .cloned()
                .ok_or_else(|| jwt_error("header must be an Object"))?,
            payload,
            key,
            if rsa { "Sha256withRSA" } else { "HmacSHA256" },
        ),
        [header, payload, key, algorithm] => (
            header
                .as_object()
                .cloned()
                .ok_or_else(|| jwt_error("header must be an Object"))?,
            payload,
            key,
            algorithm
                .as_str()
                .ok_or_else(|| jwt_error("algorithm must be a String"))?,
        ),
        _ => return Err(jwt_error(
            "expected (payload, key), (header, payload, key), or (header, payload, key, algorithm)",
        )),
    };
    if !payload.is_object() {
        return Err(jwt_error("payload must be an Object"));
    }
    let key = key
        .as_str()
        .ok_or_else(|| jwt_error("signingKey must be a String"))?;
    let bits = match (rsa, algorithm.to_ascii_uppercase().as_str()) {
        (false, "HMACSHA256") | (true, "SHA256WITHRSA") => 256,
        (false, "HMACSHA384") | (true, "SHA384WITHRSA") => 384,
        (false, "HMACSHA512") | (true, "SHA512WITHRSA") => 512,
        _ => return Err(jwt_error("unsupported signing algorithm")),
    };
    let mut header = header;
    header.insert(
        "alg".to_string(),
        Value::String(format!("{}{bits}", if rsa { "RS" } else { "HS" })),
    );
    header.insert("typ".to_string(), Value::String("JWT".to_string()));
    let input = format!(
        "{}.{}",
        encode_json(&Value::Object(header))?,
        encode_json(payload)?
    );
    let signature = if rsa {
        let key = RsaPrivateKey::from_pkcs8_pem(key)
            .or_else(|_| RsaPrivateKey::from_pkcs1_pem(key))
            .map_err(|_| jwt_error("signingKey must be an unencrypted PKCS#1 or PKCS#8 RSA private key in PEM format"))?;
        if key.n().bits() < 2048 {
            return Err(jwt_error("RSA private key must be at least 2048 bits"));
        }
        key.validate()
            .map_err(|_| jwt_error("invalid RSA private key"))?;
        let signature = match bits {
            256 => key.sign_with_rng(
                &mut OsRng,
                Pkcs1v15Sign::new::<sha2::Sha256>(),
                &sha2::Sha256::digest(input.as_bytes()),
            ),
            384 => key.sign_with_rng(
                &mut OsRng,
                Pkcs1v15Sign::new::<sha2::Sha384>(),
                &sha2::Sha384::digest(input.as_bytes()),
            ),
            _ => key.sign_with_rng(
                &mut OsRng,
                Pkcs1v15Sign::new::<sha2::Sha512>(),
                &sha2::Sha512::digest(input.as_bytes()),
            ),
        };
        signature.map_err(|_| jwt_error("RSA signing failed"))?
    } else {
        if key.len() * 8 < bits {
            return Err(jwt_error(&format!(
                "HMAC signing key must be at least {bits} bits"
            )));
        }
        hmac_bytes(
            &binary_value(key.as_bytes().to_vec()),
            &binary_value(input.as_bytes().to_vec()),
            &Value::String(format!("HmacSHA{bits}")),
        )?
    };
    Ok(Value::String(format!("{input}.{}", base64url(&signature))))
}

fn base64url(bytes: &[u8]) -> String {
    base64_encode(bytes)
        .trim_end_matches('=')
        .replace('+', "-")
        .replace('/', "_")
}

// DataWeave's JWT JSON has spaces after colons but not after commas. These
// bytes are part of the signature and must match the documented tokens.
struct JwtFormatter;

impl serde_json::ser::Formatter for JwtFormatter {
    fn begin_object_value<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        writer.write_all(b": ")
    }
}

fn encode_json(value: &Value) -> Result<String, DwError> {
    let mut bytes = Vec::new();
    value
        .serialize(&mut serde_json::Serializer::with_formatter(
            &mut bytes,
            JwtFormatter,
        ))
        .map_err(|_| jwt_error("unable to serialize header or payload"))?;
    Ok(base64url(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn matches_documented_hmac_token() {
        assert_eq!(sign("HMAC", &[
            json!({"header": "value"}),
            json!({"firstName": "Michael", "lastName": "Jones"}),
            json!("012345678901234567890123456789012345678901234567"),
            json!("HmacSHA384"),
        ]).unwrap(), json!("eyJoZWFkZXIiOiAidmFsdWUiLCJhbGciOiAiSFMzODQiLCJ0eXAiOiAiSldUIn0.eyJmaXJzdE5hbWUiOiAiTWljaGFlbCIsImxhc3ROYW1lIjogIkpvbmVzIn0.F1xP9JjXqmwVfPXULZ7F6EYu2Td36eW4xUWElGHN7ayjWIiBPWM3L4STPQzQ570R"));
    }

    #[test]
    fn rejects_weak_keys_and_cross_family_algorithms() {
        assert!(sign("HMAC", &[json!({}), json!("short")])
            .unwrap_err()
            .to_string()
            .contains("at least 256 bits"));
        assert!(sign(
            "RSA",
            &[json!({}), json!({}), json!("test"), json!("HmacSHA256")]
        )
        .unwrap_err()
        .to_string()
        .contains("unsupported signing algorithm"));
    }
}
