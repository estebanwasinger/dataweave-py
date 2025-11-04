/**
 * This module provide functions that perform encryptions through common
 * algorithms, such as MD5, SHA1, and so on.
 *
 *
 * To use this module, you must import it to your DataWeave code, for example,
 * by adding the line `import * from dw::Crypto` to the header of your
 * DataWeave script.
 */
%dw 2.0

/**
* Computes an HMAC hash (with a secret cryptographic key) on input content.
*
*
* See also, `HMACWith`.
*
* === Parameters
*
* [%header, cols="1,3a"]
* |===
* | Name | Description
* | secret | The secret cryptographic key (a binary value) used when encrypting the `content`.
* | content | The binary input value.
* | algorithm | The hashing algorithm. `HmacSHA1` is the default. Valid values depend on the
*    JDK version you are using. For JDK 8 and JDK 11, `HmacMD5`, `HmacSHA1`, `HmacSHA224`,
*   `HmacSHA256`, `HmacSHA384`, and `HmacSHA512` are valid algorithms. For JDK 11, `HmacSHA512/224`
*    and `HmacSHA512/256` are also valid.
* |===
*
* === Example
*
* This example uses HMAC with a secret value to encrypt the input content.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::Crypto
* output application/json
* ---
* {
*   "HMACBinary" : Crypto::HMACBinary("confidential" as Binary, "xxxxx" as Binary, "HmacSHA512")
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* {
*   "HMACBinary": "\ufffd\ufffd\ufffd\ufffd^h\ufffd!3\u0005\ufffd֎\u00017\ufffd\ufffd\ufffd`\ufffd8?\ufffdjn7\ufffdbs;\t\ufffdƅ\ufffd\ufffd\ufffdx&g\ufffd~\ufffd\ufffd%\ufffd7>1\ufffdK\u000e@\ufffdC\u0011\ufffdT\ufffd}W"
* }
* ----
*/
fun HMACBinary(secret: Binary, content: Binary, algorithm: String = "HmacSHA1"): Binary = native("system::HMACFunctionValue")

/**
* Computes the hash value of binary content using a specified algorithm.
*
* 
* The first argument specifies the binary content to use to calculate the hash value, and the second argument specifies the hashing algorithm to use. The second argument must be any of the accepted Algorithm names:
*
*
* [%header%autowidth.spread]
* |===
* |Algorithm names |Description
* |`MD2` |The MD2 message digest algorithm as defined in http://www.ietf.org/rfc/rfc1319.txt[RFC 1319].
* |`MD5` |The MD5 message digest algorithm as defined in http://www.ietf.org/rfc/rfc1321.txt[RFC 1321].
* |`SHA-1`, `SHA-256`, `SHA-384`, `SHA-512` | Hash algorithms defined in the http://csrc.nist.gov/publications/fips/index.html[FIPS PUB 180-2]. SHA-256 is a 256-bit hash function intended to provide 128 bits of security against collision attacks, while SHA-512 is a 512-bit hash function intended to provide 256 bits of security. A 384-bit hash may be obtained by truncating the SHA-512 output.
* |===
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | content | The binary input value to hash.
* | algorithm | The name of the algorithm to use for calculating the hash value of `content`. This value is a string. Defaults to `SHA-1`.
* |===
*
* === Example
*
* This example uses the MD2 algorithm to encrypt a binary value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::Crypto
* output application/json
* ---
* { "md2" : Crypto::hashWith("hello" as Binary, "MD2") }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "md2": "\ufffd\u0004ls\ufffd\u00031\ufffdh\ufffd}8\u0004\ufffd\u0006U" }
* ----
*/
fun hashWith(content: Binary, algorithm: String = "SHA-1"): Binary = native("system::HashFunctionValue")

/**
* Computes an HMAC hash (with a secret cryptographic key) on input content,
* then transforms the result into a lowercase, hexadecimal string.
*
*
* See also, `HMACBinary`.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | secret | The secret cryptographic key (a binary value) used when encrypting the `content`.
* | content | The binary input value.
* | algorithm | (_Introduced in DataWeave 2.2.0. Supported by Mule 4.2 and later._) The hashing algorithm. By default, `HmacSHA1` is used. Other valid values are `HmacSHA256` and `HmacSHA512`.
* |===
*
* === Example
*
* This example uses HMAC with a secret value to encrypt the input content using the `HmacSHA256` algorithm.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::Crypto
* output application/json
* ---
* { "HMACWith" : Crypto::HMACWith("secret_key" as Binary, "Some value to hash" as Binary, "HmacSHA256") }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "HMACWith": "b51b4fe8c4e37304605753272b5b4321f9644a9b09cb1179d7016c25041d1747" }
* ----
*/
fun HMACWith(secret: Binary, content: Binary, @Since(version = "2.2.0") algorithm: String = "HmacSHA1"): String =
  lower(
    dw::core::Binaries::toHex(
      HMACBinary(secret, content, algorithm)
    )
  )

/**
* Computes the MD5 hash and transforms the binary result into a
* hexadecimal lower case string.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | content | A binary input value to encrypt.
* |===
*
* === Example
*
* This example uses the MD5 algorithm to encrypt a binary value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::Crypto
* output application/json
* ---
* { "md5" : Crypto::MD5("asd" as Binary) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "md5": "7815696ecbf1c96e6894b779456d330e" }
* ----
*/
fun MD5(content: Binary): String =
  lower(
    dw::core::Binaries::toHex(content hashWith "MD5")
   )

/**
* Computes the SHA1 hash and transforms the result into a hexadecimal,
* lowercase string.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | content | A binary input value to encrypt.
* |===
*
* === Example
*
* This example uses the SHA1 algorithm to encrypt a binary value.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::Crypto
* output application/json
* ---
* { "sha1" : Crypto::SHA1("dsasd" as Binary) }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "sha1": "2fa183839c954e6366c206367c9be5864e4f4a65" }
* ----
*/
fun SHA1(content: Binary): String =
  lower(
    dw::core::Binaries::toHex(
      content hashWith "SHA1"
    )
   )
