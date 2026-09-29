// Test-only RS256 signing material for IX JWT harnesses.
//
// Keys are minted at runtime with the OpenSSL CLI so `appfw-runtime` tests can
// sign RS256 JWTs and serve a matching JWKS document without the `rsa` crate
// (RUSTSEC-2023-0071 / Marvin) and without committing a private key. Material
// stays in process memory and must not be reused outside these tests.

mod ix_rs256_test_material {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::sync::OnceLock;

    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine as _;

    struct MintedRs256 {
        pem: String,
        modulus: String,
        exponent: String,
    }

    static MINTED: OnceLock<MintedRs256> = OnceLock::new();

    fn minted() -> &'static MintedRs256 {
        MINTED.get_or_init(mint_rs256_with_openssl)
    }

    fn mint_rs256_with_openssl() -> MintedRs256 {
        let generated = Command::new("openssl")
            .args(["genrsa", "2048"])
            .output()
            .expect("openssl genrsa must be available to remint RS256 test keys");
        if !generated.status.success() {
            panic!(
                "openssl genrsa failed: {}",
                String::from_utf8_lossy(&generated.stderr)
            );
        }
        let pem = String::from_utf8(generated.stdout).expect("openssl genrsa PEM utf8");
        if !pem.contains("BEGIN") || !pem.contains("PRIVATE KEY") {
            panic!("openssl genrsa did not emit a private-key PEM");
        }

        let modulus_line = openssl_rsa_stdout(&pem, &["-modulus"]);
        let modulus_hex = modulus_line.trim().trim_start_matches("Modulus=");
        let modulus = jwks_unsigned_int(decode_hex(modulus_hex));

        let rsa_text = openssl_rsa_stdout(&pem, &["-text"]);
        let exponent = jwks_exponent_from_rsa_text(&rsa_text);

        MintedRs256 {
            pem,
            modulus,
            exponent,
        }
    }

    fn openssl_rsa_stdout(pem: &str, extra_args: &[&str]) -> String {
        let mut child = Command::new("openssl")
            .arg("rsa")
            .arg("-noout")
            .args(extra_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("openssl rsa must be available to derive JWKS n/e");
        {
            let mut stdin = child.stdin.take().expect("openssl rsa stdin");
            stdin
                .write_all(pem.as_bytes())
                .expect("write generated PEM to openssl rsa");
        }
        let output = child.wait_with_output().expect("openssl rsa should exit");
        if !output.status.success() {
            panic!(
                "openssl rsa {:?} failed: {}",
                extra_args,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        String::from_utf8(output.stdout).expect("openssl rsa stdout utf8")
    }

    fn jwks_exponent_from_rsa_text(text: &str) -> String {
        for line in text.lines() {
            let trimmed = line.trim();
            let rest = match trimmed.strip_prefix("publicExponent:") {
                Some(rest) => rest,
                None => continue,
            };
            if let Some((_, hex)) = rest.split_once("(0x") {
                let hex = hex.trim().trim_end_matches(')');
                return jwks_unsigned_int(decode_hex(hex));
            }
            let decimal = rest
                .split_whitespace()
                .next()
                .expect("publicExponent decimal");
            let value: u64 = decimal
                .parse()
                .expect("publicExponent should be an integer");
            return jwks_unsigned_int(value.to_be_bytes().to_vec());
        }
        panic!("openssl rsa -text did not report publicExponent");
    }

    fn decode_hex(hex: &str) -> Vec<u8> {
        let hex = hex.trim();
        let padded = if hex.len() % 2 == 1 {
            format!("0{hex}")
        } else {
            hex.to_string()
        };
        (0..padded.len())
            .step_by(2)
            .map(|index| {
                u8::from_str_radix(&padded[index..index + 2], 16)
                    .unwrap_or_else(|_| panic!("invalid openssl hex at {index}"))
            })
            .collect()
    }

    fn jwks_unsigned_int(bytes: Vec<u8>) -> String {
        let mut start = 0;
        while start + 1 < bytes.len() && bytes[start] == 0 {
            start += 1;
        }
        URL_SAFE_NO_PAD.encode(&bytes[start..])
    }

    pub fn encoding_key() -> jsonwebtoken::EncodingKey {
        jsonwebtoken::EncodingKey::from_rsa_pem(minted().pem.as_bytes())
            .expect("runtime-minted RS256 encoding key")
    }

    pub fn jwks(kid: &str) -> serde_json::Value {
        let minted = minted();
        serde_json::json!({
            "keys": [{
                "kty": "RSA",
                "alg": "RS256",
                "kid": kid,
                "use": "sig",
                "e": minted.exponent,
                "n": minted.modulus
            }]
        })
    }
}

#[allow(dead_code)]
pub fn ix_rs256_test_encoding_key() -> jsonwebtoken::EncodingKey {
    ix_rs256_test_material::encoding_key()
}

#[allow(dead_code)]
pub fn ix_rs256_test_jwks(kid: &str) -> serde_json::Value {
    ix_rs256_test_material::jwks(kid)
}
