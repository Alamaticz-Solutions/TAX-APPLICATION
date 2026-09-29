#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "dormant internal domain-change foundation; remove with first reviewed non-test consumer or public-adapter reslice"
    )
)]

use std::sync::OnceLock;

use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine as _,
};
use regex::Regex;

const MAX_PRIVATE_KEY_INPUT_BYTES: usize = 32_768;
const MIN_ENCODED_KEY_BYTES: usize = 24;
const MAX_ENCODED_KEY_BYTES: usize = 22_000;
const MIN_DECODED_KEY_BYTES: usize = 16;
const MAX_DECODED_KEY_BYTES: usize = 16_384;

const ASSIGNMENT_MARKERS: [&str; 9] = [
    "bearer ",
    "password=",
    "passwd=",
    "secret=",
    "token=",
    "api_key=",
    "api-key=",
    "apikey=",
    "private_key=",
];

const PRIVATE_PEM_LABELS: [&str; 5] = [
    "PRIVATE KEY",
    "ENCRYPTED PRIVATE KEY",
    "RSA PRIVATE KEY",
    "EC PRIVATE KEY",
    "OPENSSH PRIVATE KEY",
];

/// Bounded defense-in-depth classification for internal metadata fields.
///
/// This intentionally does not claim complete secret, credential, PHI, or PII
/// detection. Callers receive only a Boolean classification and must not log
/// or otherwise retain the inspected value.
pub(crate) fn contains_sensitive_metadata(value: &str) -> bool {
    contains_assignment_marker(value)
        || contains_provider_token(value)
        || contains_jwt(value)
        || contains_email(value)
        || contains_ssn(value)
        || contains_encoded_private_key(value)
}

fn contains_assignment_marker(value: &str) -> bool {
    let lowercase = value.to_ascii_lowercase();
    ASSIGNMENT_MARKERS
        .iter()
        .any(|marker| lowercase.contains(marker))
}

fn contains_provider_token(value: &str) -> bool {
    let bytes = value.as_bytes();
    has_prefixed_token(
        bytes,
        &[b"ghp_", b"gho_", b"ghu_", b"ghs_", b"ghr_"],
        is_ascii_alphanumeric,
        is_ascii_alphanumeric_or_underscore,
        20,
        None,
    ) || has_prefixed_token(
        bytes,
        &[b"github_pat_"],
        is_ascii_alphanumeric_or_underscore,
        is_ascii_alphanumeric_or_underscore,
        20,
        None,
    ) || has_prefixed_token(
        bytes,
        &[b"xoxb-", b"xoxa-", b"xoxp-", b"xoxr-", b"xoxs-"],
        is_ascii_alphanumeric_or_hyphen,
        is_ascii_alphanumeric_or_hyphen,
        20,
        None,
    ) || has_prefixed_token(bytes, &[b"sk-"], is_base64url, is_base64url, 20, None)
        || has_prefixed_token(
            bytes,
            &[b"AKIA", b"ASIA"],
            is_ascii_uppercase_or_digit,
            is_ascii_alphanumeric,
            16,
            Some(16),
        )
        || has_prefixed_token(bytes, &[b"AIza"], is_base64url, is_base64url, 30, None)
        || has_prefixed_token(
            bytes,
            &[b"sk_live_", b"rk_live_"],
            is_ascii_alphanumeric,
            is_ascii_alphanumeric_or_underscore,
            16,
            None,
        )
}

fn has_prefixed_token(
    value: &[u8],
    prefixes: &[&[u8]],
    body_class: fn(u8) -> bool,
    boundary_class: fn(u8) -> bool,
    minimum_body_len: usize,
    fixed_body_len: Option<usize>,
) -> bool {
    for start in 0..value.len() {
        if start > 0 && boundary_class(value[start - 1]) {
            continue;
        }
        for prefix in prefixes {
            let Some(body_start) = start.checked_add(prefix.len()) else {
                continue;
            };
            if body_start > value.len() || &value[start..body_start] != *prefix {
                continue;
            }
            let mut end = body_start;
            while end < value.len() && body_class(value[end]) {
                end += 1;
            }
            let body_len = end - body_start;
            if body_len < minimum_body_len || fixed_body_len.is_some_and(|len| body_len != len) {
                continue;
            }
            if end < value.len() && boundary_class(value[end]) {
                continue;
            }
            return true;
        }
    }
    false
}

fn contains_jwt(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut start = 0;
    while start < bytes.len() {
        while start < bytes.len() && !is_jwt_candidate_byte(bytes[start]) {
            start += 1;
        }
        let mut end = start;
        while end < bytes.len() && is_jwt_candidate_byte(bytes[end]) {
            end += 1;
        }
        if start < end && jwt_candidate_is_sensitive(&value[start..end]) {
            return true;
        }
        start = end.saturating_add(1);
    }
    false
}

fn jwt_candidate_is_sensitive(candidate: &str) -> bool {
    let mut segments = candidate.split('.');
    let (Some(header), Some(payload), Some(signature), None) = (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) else {
        return false;
    };
    if header.len() < 8
        || payload.len() < 2
        || signature.len() < 8
        || [header, payload, signature]
            .iter()
            .any(|segment| segment.is_empty() || !segment.bytes().all(is_base64url))
    {
        return false;
    }
    let Ok(decoded) = URL_SAFE_NO_PAD.decode(header) else {
        return false;
    };
    let Ok(header_json) = serde_json::from_slice::<serde_json::Value>(&decoded) else {
        return false;
    };
    header_json
        .as_object()
        .and_then(|object| object.get("alg"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|algorithm| !algorithm.is_empty())
}

fn contains_email(value: &str) -> bool {
    let bytes = value.as_bytes();
    for at in bytes
        .iter()
        .enumerate()
        .filter_map(|(index, byte)| (*byte == b'@').then_some(index))
    {
        let mut start = at;
        while start > 0 && is_email_local(bytes[start - 1]) {
            start -= 1;
        }
        let mut end = at + 1;
        while end < bytes.len() && is_email_domain(bytes[end]) {
            end += 1;
        }
        let local = &bytes[start..at];
        let domain = &bytes[at + 1..end];
        if local.is_empty()
            || local.len() > 64
            || domain.is_empty()
            || end - start > 254
            || (start > 0 && is_email_local(bytes[start - 1]))
            || (end < bytes.len() && is_ascii_alphanumeric_or_hyphen(bytes[end]))
        {
            continue;
        }
        let Some(labels) = parse_email_domain_labels(domain) else {
            continue;
        };
        let Some(final_label) = labels.last() else {
            continue;
        };
        if labels.len() >= 2
            && (2..=63).contains(&final_label.len())
            && final_label.iter().all(|byte| byte.is_ascii_alphabetic())
        {
            return true;
        }
    }
    false
}

fn parse_email_domain_labels(domain: &[u8]) -> Option<Vec<&[u8]>> {
    let labels = domain.split(|byte| *byte == b'.').collect::<Vec<_>>();
    for label in &labels {
        if label.is_empty()
            || label.len() > 63
            || !label[0].is_ascii_alphanumeric()
            || !label[label.len() - 1].is_ascii_alphanumeric()
            || !label
                .iter()
                .all(|byte| is_ascii_alphanumeric_or_hyphen(*byte))
        {
            return None;
        }
    }
    Some(labels)
}

fn contains_ssn(value: &str) -> bool {
    let bytes = value.as_bytes();
    for start in 0..bytes.len().saturating_sub(10) {
        let end = start + 11;
        if start > 0 && is_ascii_alphanumeric_or_underscore(bytes[start - 1]) {
            continue;
        }
        if end < bytes.len() && is_ascii_alphanumeric_or_underscore(bytes[end]) {
            continue;
        }
        let candidate = &bytes[start..end];
        if candidate[0..3].iter().all(|byte| byte.is_ascii_digit())
            && candidate[3] == b'-'
            && candidate[4..6].iter().all(|byte| byte.is_ascii_digit())
            && candidate[6] == b'-'
            && candidate[7..11].iter().all(|byte| byte.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

pub(crate) fn contains_encoded_private_key(value: &str) -> bool {
    if value.len() > MAX_PRIVATE_KEY_INPUT_BYTES {
        return false;
    }
    if contains_exact_private_pem_marker(value) || decoded_candidate_is_private_key(value) {
        return true;
    }
    embedded_der_private_key_candidate(value)
        || contains_encoded_openssh_magic(value)
        || encoded_candidate_regex()
            .find_iter(value)
            .any(|candidate| decoded_candidate_is_private_key(candidate.as_str()))
}

fn contains_exact_private_pem_marker(value: &str) -> bool {
    PRIVATE_PEM_LABELS.iter().any(|label| {
        value.contains(&format!("-----BEGIN {label}-----"))
            || value.contains(&format!("-----END {label}-----"))
    })
}

fn decoded_candidate_is_private_key(value: &str) -> bool {
    let normalized = value
        .bytes()
        .filter(|byte| !is_ascii_key_whitespace(*byte))
        .collect::<Vec<_>>();
    let Some(decoded) = decode_canonical_candidate(&normalized) else {
        return false;
    };
    looks_like_private_key_bytes(&decoded)
}

fn decode_canonical_candidate(normalized: &[u8]) -> Option<Vec<u8>> {
    if !(MIN_ENCODED_KEY_BYTES..=MAX_ENCODED_KEY_BYTES).contains(&normalized.len())
        || !normalized.iter().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'_' | b'-' | b'=')
        })
    {
        return None;
    }
    let supplied_padding = normalized
        .iter()
        .rev()
        .take_while(|byte| **byte == b'=')
        .count();
    if supplied_padding > 2 || normalized[..normalized.len() - supplied_padding].contains(&b'=') {
        return None;
    }

    let mut unpadded = normalized[..normalized.len() - supplied_padding]
        .iter()
        .map(|byte| match byte {
            b'-' => b'+',
            b'_' => b'/',
            other => *other,
        })
        .collect::<Vec<_>>();
    if unpadded.len() % 4 == 1 {
        return None;
    }
    let unpadded_input = unpadded.clone();
    unpadded.extend(std::iter::repeat_n(b'=', (4 - unpadded.len() % 4) % 4));
    let decoded = STANDARD.decode(&unpadded).ok()?;
    if !(MIN_DECODED_KEY_BYTES..=MAX_DECODED_KEY_BYTES).contains(&decoded.len()) {
        return None;
    }
    if STANDARD.encode(&decoded).trim_end_matches('=').as_bytes() != unpadded_input {
        return None;
    }
    Some(decoded)
}

fn looks_like_private_key_bytes(bytes: &[u8]) -> bool {
    if bytes.starts_with(b"openssh-key-v1\0") {
        return true;
    }
    let Some((0x30, content, consumed)) = parse_der_tlv(bytes) else {
        return false;
    };
    if consumed != bytes.len() {
        return false;
    }
    let Some(children) = parse_der_children(content) else {
        return false;
    };
    pkcs8_private_shape(&children)
        || pkcs1_rsa_private_shape(&children)
        || sec1_ec_private_shape(&children)
}

fn pkcs8_private_shape(children: &[(u8, &[u8])]) -> bool {
    children.len() >= 3
        && children[0].0 == 0x02
        && matches!(children[0].1, [0] | [1])
        && children[1].0 == 0x30
        && algorithm_identifier_is_well_formed(children[1].1)
        && children[2].0 == 0x04
        && children[3..]
            .iter()
            .all(|(tag, _)| matches!(tag, 0x81 | 0xa0 | 0xa1))
}

fn pkcs1_rsa_private_shape(children: &[(u8, &[u8])]) -> bool {
    (children.len() == 9 || children.len() == 10 && children[9].0 == 0x30)
        && children[..9].iter().all(|(tag, _)| *tag == 0x02)
        && matches!(children[0].1, [0] | [1])
}

fn sec1_ec_private_shape(children: &[(u8, &[u8])]) -> bool {
    children.len() >= 2
        && children[0].0 == 0x02
        && children[0].1 == [1]
        && children[1].0 == 0x04
        && children[2..]
            .iter()
            .all(|(tag, _)| matches!(tag, 0xa0 | 0xa1))
}

fn parse_der_tlv(bytes: &[u8]) -> Option<(u8, &[u8], usize)> {
    if bytes.len() < 2 {
        return None;
    }
    let tag = bytes[0];
    let first_length = bytes[1];
    if first_length < 0x80 {
        let end = usize::from(first_length).checked_add(2)?;
        if end > bytes.len() {
            return None;
        }
        return Some((tag, &bytes[2..end], end));
    }
    let length_bytes = usize::from(first_length & 0x7f);
    if length_bytes == 0 || length_bytes > 4 || bytes.len() < length_bytes + 2 {
        return None;
    }
    let declared = bytes[2..2 + length_bytes]
        .iter()
        .try_fold(0usize, |length, byte| {
            length.checked_mul(256)?.checked_add(usize::from(*byte))
        })?;
    let content_offset = length_bytes + 2;
    let end = content_offset.checked_add(declared)?;
    if end > bytes.len() {
        return None;
    }
    Some((tag, &bytes[content_offset..end], end))
}

fn parse_der_children(mut bytes: &[u8]) -> Option<Vec<(u8, &[u8])>> {
    let mut children = Vec::new();
    while !bytes.is_empty() {
        let (tag, content, consumed) = parse_der_tlv(bytes)?;
        children.push((tag, content));
        bytes = &bytes[consumed..];
    }
    Some(children)
}

fn algorithm_identifier_is_well_formed(content: &[u8]) -> bool {
    parse_der_children(content).is_some_and(|children| {
        matches!(children.as_slice(), [(0x06, oid)] if !oid.is_empty())
            || matches!(children.as_slice(), [(0x06, oid), _] if !oid.is_empty())
    })
}

fn embedded_der_private_key_candidate(value: &str) -> bool {
    let normalized = value
        .bytes()
        .filter(|byte| !is_ascii_key_whitespace(*byte))
        .map(|byte| match byte {
            b'-' => b'+',
            b'_' => b'/',
            other => other,
        })
        .collect::<Vec<_>>();
    for start in normalized
        .iter()
        .enumerate()
        .filter_map(|(index, byte)| (*byte == b'M').then_some(index))
    {
        if normalized.len().saturating_sub(start) < 8 {
            continue;
        }
        let Ok(prefix) = STANDARD.decode(&normalized[start..start + 8]) else {
            continue;
        };
        let Some(total_bytes) = der_sequence_total_length(&prefix) else {
            continue;
        };
        let encoded_len = total_bytes.div_ceil(3) * 4
            - usize::from(total_bytes % 3 != 0)
            - usize::from(total_bytes % 3 == 1);
        let Some(end) = start.checked_add(encoded_len) else {
            continue;
        };
        if end <= normalized.len()
            && (end == normalized.len() || normalized[end] != b'=')
            && decoded_candidate_is_private_key(
                std::str::from_utf8(&normalized[start..end]).unwrap_or_default(),
            )
        {
            return true;
        }
    }
    false
}

fn der_sequence_total_length(prefix: &[u8]) -> Option<usize> {
    if prefix.len() < 2 || prefix[0] != 0x30 {
        return None;
    }
    let first_length = prefix[1];
    if first_length < 0x80 {
        return usize::from(first_length).checked_add(2);
    }
    let length_bytes = usize::from(first_length & 0x7f);
    if length_bytes == 0 || length_bytes > 4 || prefix.len() < length_bytes + 2 {
        return None;
    }
    let declared = prefix[2..2 + length_bytes]
        .iter()
        .try_fold(0usize, |length, byte| {
            length.checked_mul(256)?.checked_add(usize::from(*byte))
        })?;
    declared.checked_add(length_bytes + 2)
}

fn contains_encoded_openssh_magic(value: &str) -> bool {
    let normalized = value
        .bytes()
        .filter(|byte| !is_ascii_key_whitespace(*byte))
        .map(|byte| match byte {
            b'-' => b'+',
            b'_' => b'/',
            other => other,
        })
        .collect::<Vec<_>>();
    normalized
        .windows(b"b3BlbnNzaC1rZXktdjEA".len())
        .any(|window| window == b"b3BlbnNzaC1rZXktdjEA")
}

fn encoded_candidate_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"[A-Za-z0-9+/_=\-\t\n\r\x0B\x0C ]{40,}")
            .expect("static encoded-key candidate regex")
    })
}

fn is_ascii_key_whitespace(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | 0x0b | 0x0c | b'\r' | b' ')
}

fn is_ascii_alphanumeric(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
}

fn is_ascii_alphanumeric_or_underscore(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_ascii_alphanumeric_or_hyphen(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

fn is_ascii_uppercase_or_digit(byte: u8) -> bool {
    byte.is_ascii_uppercase() || byte.is_ascii_digit()
}

fn is_base64url(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

fn is_jwt_candidate_byte(byte: u8) -> bool {
    is_base64url(byte) || byte == b'.'
}

fn is_email_local(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'.' | b'!'
                | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'/'
                | b'='
                | b'?'
                | b'^'
                | b'_'
                | b'`'
                | b'{'
                | b'|'
                | b'}'
                | b'~'
                | b'-'
        )
}

fn is_email_domain(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.')
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPACT_ED25519_PKCS8: &str =
        "MC4CAQAwBQYDK2VwBCIEIJiGLkWAlHdIeMza9cdqgW0p86icRRK7WydQw1GJH1cJ";
    const ED25519_SPKI_PUBLIC_KEY: &str =
        "MCowBQYDK2VwAyEA9cdrWezmZsInlGpAz/hBoKuRQEfqNKduwdsqbc38HM4=";

    fn der_length(length: usize) -> Vec<u8> {
        if length < 0x80 {
            return vec![length as u8];
        }
        let bytes = length.to_be_bytes();
        let first = bytes.iter().position(|byte| *byte != 0).unwrap();
        let significant = &bytes[first..];
        let mut encoded = vec![0x80 | significant.len() as u8];
        encoded.extend_from_slice(significant);
        encoded
    }

    fn der_tlv(tag: u8, content: &[u8]) -> Vec<u8> {
        let mut encoded = vec![tag];
        encoded.extend(der_length(content.len()));
        encoded.extend_from_slice(content);
        encoded
    }

    fn der_sequence(children: &[Vec<u8>]) -> Vec<u8> {
        let content = children.iter().flatten().copied().collect::<Vec<_>>();
        der_tlv(0x30, &content)
    }

    fn algorithm_identifier(parameters: &[Vec<u8>]) -> Vec<u8> {
        let mut children = vec![der_tlv(0x06, &[0x2a])];
        children.extend_from_slice(parameters);
        der_sequence(&children)
    }

    fn pkcs8_fixture(version: &[u8], private_octets: usize) -> Vec<u8> {
        der_sequence(&[
            der_tlv(0x02, version),
            algorithm_identifier(&[]),
            der_tlv(0x04, &vec![0x5a; private_octets]),
        ])
    }

    fn pkcs1_fixture(version: u8) -> Vec<u8> {
        let mut children = vec![der_tlv(0x02, &[version])];
        for value in [3, 5, 7, 11, 13, 17, 19, 23] {
            children.push(der_tlv(0x02, &[value]));
        }
        der_sequence(&children)
    }

    fn sec1_fixture(version: u8) -> Vec<u8> {
        der_sequence(&[der_tlv(0x02, &[version]), der_tlv(0x04, &[0x5a; 16])])
    }

    fn assert_private_classification(label: &str, value: &str, expected: bool) {
        assert_eq!(
            contains_encoded_private_key(value),
            expected,
            "private-key classification mismatch for {label}"
        );
    }

    #[test]
    fn sd_n01_assignment_markers_are_ascii_case_insensitive() {
        for marker in ASSIGNMENT_MARKERS {
            assert!(contains_sensitive_metadata(&format!(
                "prefix{marker}suffix"
            )));
            assert!(contains_sensitive_metadata(&format!(
                "prefix{}suffix",
                marker.to_ascii_uppercase()
            )));
        }
        assert!(!contains_sensitive_metadata("ordinary opaque metadata"));
    }

    #[test]
    fn sd_n02_provider_tokens_obey_lengths_and_boundaries() {
        let cases = [
            ("ghp_", "A", 20, false),
            ("github_pat_", "A_", 20, false),
            ("xoxb-", "A-", 20, false),
            ("sk-", "A_", 20, false),
            ("AKIA", "A", 16, true),
            ("AIza", "A_", 30, false),
            ("sk_live_", "A", 16, false),
        ];
        for (prefix, alphabet, minimum, fixed) in cases {
            let body = alphabet
                .repeat(minimum)
                .chars()
                .take(minimum)
                .collect::<String>();
            let short = alphabet
                .repeat(minimum)
                .chars()
                .take(minimum - 1)
                .collect::<String>();
            assert!(!contains_provider_token(&format!("{prefix}{short}")));
            assert!(contains_provider_token(&format!("({prefix}{body})")));
            assert!(!contains_provider_token(&format!("A{prefix}{body}")));
            if fixed {
                assert!(!contains_provider_token(&format!("{prefix}{body}A")));
            } else {
                assert!(contains_provider_token(&format!("{prefix}{body}A")));
            }
            let mut changed = prefix.to_string();
            changed.replace_range(0..1, &prefix[0..1].to_ascii_lowercase());
            if changed != prefix {
                assert!(!contains_provider_token(&format!("{changed}{body}")));
            }
        }
    }

    #[test]
    fn sd_n03_jwt_requires_three_valid_unpadded_segments_and_alg() {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
        let token = format!("{header}.e30.c2lnbmF0dXJl");
        assert!(contains_jwt(&token));
        assert!(contains_jwt(&format!("({token})")));
        for invalid in [
            "alpha.beta.gamma",
            &format!("{header}.e30"),
            &format!("{header}.e30.c2lnbmF0dXJl.extra"),
            &format!("{header}=.e30.c2lnbmF0dXJl"),
            &format!(
                "{}.e30.c2lnbmF0dXJl",
                URL_SAFE_NO_PAD.encode(br#"{"typ":"JWT"}"#)
            ),
            &format!(
                "{}.e30.c2lnbmF0dXJl",
                URL_SAFE_NO_PAD.encode(br#"{"alg":""}"#)
            ),
        ] {
            assert!(!contains_jwt(invalid));
        }
        assert!(!contains_jwt(&format!("A{token}")));
    }

    #[test]
    fn sd_n04_email_and_ssn_boundaries_are_exact() {
        for sensitive in [
            "person@example.com",
            "(person+tag@example-domain.health)",
            "123-45-6789",
            "(123-45-6789)",
        ] {
            assert!(contains_sensitive_metadata(sensitive));
        }
        for allowed in [
            "person@example",
            "person@example.c",
            "person@-example.com",
            "person@example-.com",
            "A123-45-6789",
            "123-45-6789_",
            "12-345-6789",
        ] {
            assert!(!contains_email(allowed) && !contains_ssn(allowed));
        }
    }

    #[test]
    fn sd_n05_exact_private_pem_markers_deny_without_decoding() {
        for label in PRIVATE_PEM_LABELS {
            assert!(contains_encoded_private_key(&format!(
                "-----BEGIN {label}-----"
            )));
            assert!(contains_encoded_private_key(&format!(
                "-----END {label}-----"
            )));
            assert!(!contains_encoded_private_key(&format!(
                "-----BEGIN {}-----",
                label.to_ascii_lowercase()
            )));
        }
        assert!(!contains_encoded_private_key("-----BEGIN PUBLIC KEY-----"));
        assert!(!contains_encoded_private_key("-----BEGIN CERTIFICATE-----"));
    }

    #[test]
    fn sd_n06_pkcs8_encodings_and_embedding_are_detected() {
        assert!(contains_encoded_private_key(COMPACT_ED25519_PKCS8));
        assert!(contains_encoded_private_key(&format!(
            "evidence:{COMPACT_ED25519_PKCS8}"
        )));
        assert!(contains_encoded_private_key(&format!(
            "{}\n{}",
            &COMPACT_ED25519_PKCS8[..32],
            &COMPACT_ED25519_PKCS8[32..]
        )));
        assert!(contains_encoded_private_key(
            &COMPACT_ED25519_PKCS8.replace('+', "-").replace('/', "_")
        ));
        assert!(contains_encoded_private_key(&format!(
            "{COMPACT_ED25519_PKCS8}A"
        )));
    }

    #[test]
    fn sd_n07_pkcs1_sec1_and_openssh_shapes_are_detected() {
        let mut pkcs1 = vec![0x30, 0x1b];
        for value in [0, 3, 5, 7, 11, 13, 17, 19, 23] {
            pkcs1.extend([0x02, 0x01, value]);
        }
        assert!(contains_encoded_private_key(&STANDARD.encode(pkcs1)));

        let mut sec1 = vec![0x30, 0x25, 0x02, 0x01, 0x01, 0x04, 0x20];
        sec1.extend([0x5a; 32]);
        assert!(contains_encoded_private_key(&STANDARD.encode(sec1)));

        let mut openssh = b"openssh-key-v1\0".to_vec();
        openssh.extend([0x42; 32]);
        assert!(contains_encoded_private_key(&STANDARD.encode(openssh)));
        assert!(contains_encoded_private_key("b3BlbnNzaC1rZXktdjEA"));
    }

    #[test]
    fn sd_n08_public_certificate_and_generic_der_are_allowed() {
        assert!(!contains_encoded_private_key(ED25519_SPKI_PUBLIC_KEY));
        let certificate = [
            0x30, 0x1c, 0x30, 0x0c, 0x02, 0x01, 0x02, 0x04, 0x07, b'C', b'E', b'R', b'T', b'D',
            b'A', b'T', 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x05, 0x00, 0x01, 0x02,
            0x03, 0x04,
        ];
        assert!(!contains_encoded_private_key(&STANDARD.encode(certificate)));
        assert!(!contains_encoded_private_key(
            &STANDARD.encode([0x30, 0x06, 0x04, 0x04, 1, 2, 3, 4,])
        ));
    }

    #[test]
    fn sd_n09_malformed_and_noncanonical_encodings_are_allowed() {
        for candidate in [
            "A".repeat(23),
            format!("{COMPACT_ED25519_PKCS8}==="),
            format!(
                "{}={}",
                &COMPACT_ED25519_PKCS8[..20],
                &COMPACT_ED25519_PKCS8[20..]
            ),
            "!".repeat(40),
            STANDARD.encode([0x30, 0x80, 0x02, 0x01, 0x00, 0x00, 0x00]),
            STANDARD.encode([0x30, 0x10, 0x02, 0x01, 0x00]),
            STANDARD.encode([0x30, 0x03, 0x02, 0x01, 0x00, 0xff]),
        ] {
            assert!(!contains_encoded_private_key(&candidate), "{candidate}");
        }
    }

    #[test]
    fn private_key_candidate_size_padding_and_malformed_controls_are_exact() {
        for allowed in [
            "A".repeat(23),
            "A".repeat(24),
            "A".repeat(22_000),
            "A".repeat(22_001),
            format!("{COMPACT_ED25519_PKCS8}==="),
            format!(
                "{}={}",
                &COMPACT_ED25519_PKCS8[..20],
                &COMPACT_ED25519_PKCS8[20..]
            ),
        ] {
            assert!(
                !contains_encoded_private_key(&allowed),
                "length {}",
                allowed.len()
            );
        }
        assert!(contains_encoded_private_key(COMPACT_ED25519_PKCS8));
        assert!(contains_encoded_private_key(&format!(
            "({COMPACT_ED25519_PKCS8})"
        )));
        assert!(contains_encoded_private_key(&format!(
            "{COMPACT_ED25519_PKCS8}A"
        )));
    }

    #[test]
    fn private_key_r4_r5_complete_outcome_matrix() {
        for label in PRIVATE_PEM_LABELS {
            for direction in ["BEGIN", "END"] {
                assert_private_classification(
                    &format!("exact {direction} {label}"),
                    &format!("-----{direction} {label}-----"),
                    true,
                );
                assert_private_classification(
                    &format!("lowercase {direction} {label}"),
                    &format!(
                        "-----{} {}-----",
                        direction.to_ascii_lowercase(),
                        label.to_ascii_lowercase()
                    ),
                    false,
                );
                assert_private_classification(
                    &format!("mixed-case {direction} {label}"),
                    &format!("-----{direction} Private Key-----"),
                    false,
                );
            }
        }
        for allowed in [
            "-----BEGIN PRIVATE KEY MATERIAL-----",
            "----BEGIN PRIVATE KEY-----",
            "-----BEGIN PUBLIC KEY-----",
            "-----END PUBLIC KEY-----",
            "-----BEGIN CERTIFICATE-----",
            "-----END CERTIFICATE-----",
            "BEGIN PRIVATE KEY",
        ] {
            assert_private_classification("nonexact PEM marker", allowed, false);
        }

        let pkcs8 = pkcs8_fixture(&[0], 32);
        let pkcs1 = pkcs1_fixture(0);
        let sec1 = sec1_fixture(1);
        for (shape, fixture) in [("PKCS#8", &pkcs8), ("PKCS#1", &pkcs1), ("SEC1", &sec1)] {
            assert!(looks_like_private_key_bytes(fixture), "{shape}");
            let padded = STANDARD.encode(fixture);
            let unpadded = padded.trim_end_matches('=');
            let url = URL_SAFE_NO_PAD.encode(fixture);
            let split = unpadded.len() / 2;
            for (form, candidate) in [
                ("standard", padded.clone()),
                ("unpadded", unpadded.to_string()),
                ("base64url", url),
                (
                    "all allowed whitespace",
                    format!(
                        "{}\t\n\u{000b}\u{000c}\r {}",
                        &unpadded[..split],
                        &unpadded[split..]
                    ),
                ),
                ("text embedded", format!("evidence:{unpadded}:complete")),
                ("punctuation bounded", format!("({unpadded})")),
            ] {
                assert_private_classification(&format!("{shape} {form}"), &candidate, true);
            }
            assert_private_classification(
                &format!("{shape} DER-prefix extraction"),
                &format!("{unpadded}A"),
                true,
            );
        }

        let mut openssh = b"openssh-key-v1\0".to_vec();
        openssh.push(0x42);
        assert_eq!(openssh.len(), 16);
        assert_private_classification("decoded OpenSSH prefix", &STANDARD.encode(&openssh), true);
        assert_private_classification("exact encoded OpenSSH magic", "b3BlbnNzaC1rZXktdjEA", true);
        assert_private_classification(
            "embedded encoded OpenSSH magic",
            "prefix:b3BlbnNzaC1rZXktdjEA:suffix",
            true,
        );
        assert_private_classification("mutated OpenSSH magic", "b3BlbnNzaC1rZXktdjEB", false);
        let mut shifted_openssh = b"xopenssh-key-v1\0".to_vec();
        assert_eq!(shifted_openssh.len(), 16);
        assert!(!looks_like_private_key_bytes(&shifted_openssh));
        shifted_openssh[0] = b'x';
        assert_private_classification(
            "decoded OpenSSH magic not at byte zero",
            &STANDARD.encode(shifted_openssh),
            false,
        );

        for (decoded_size, private_octets, expected) in [
            (15usize, 3usize, false),
            (16, 4, true),
            (16_384, 16_368, true),
            (16_385, 16_369, false),
        ] {
            let fixture = pkcs8_fixture(&[0], private_octets);
            assert_eq!(fixture.len(), decoded_size);
            assert!(looks_like_private_key_bytes(&fixture));
            assert_private_classification(
                &format!("decoded-size boundary {decoded_size}"),
                &STANDARD.encode(fixture),
                expected,
            );
        }

        let valid_sixteen = pkcs8_fixture(&[0], 4);
        assert_eq!(valid_sixteen.len(), 16);
        assert_eq!(STANDARD.encode(&valid_sixteen).len(), 24);
        assert_private_classification(
            "encoded minimum containing valid private shape",
            &STANDARD.encode(&valid_sixteen),
            true,
        );
        let mut noncanonical = STANDARD.encode(&valid_sixteen).into_bytes();
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let significant = noncanonical.iter().rposition(|byte| *byte != b'=').unwrap();
        let index = alphabet
            .iter()
            .position(|byte| *byte == noncanonical[significant])
            .unwrap();
        noncanonical[significant] = alphabet[if index % 16 == 15 {
            index - 1
        } else {
            index + 1
        }];
        let noncanonical = String::from_utf8(noncanonical).unwrap();
        assert!(STANDARD.decode(&noncanonical).is_err());
        assert_private_classification("noncanonical trailing Base64 bits", &noncanonical, false);
        for (length, expected_decoder_result) in [
            (23usize, false),
            (24, true),
            (22_000, false),
            (22_001, false),
        ] {
            let candidate = "A".repeat(length);
            assert_private_classification(
                &format!("encoded-size boundary {length}"),
                &candidate,
                false,
            );
            assert_eq!(
                decode_canonical_candidate(candidate.as_bytes()).is_some(),
                expected_decoder_result,
                "decoder boundary {length}"
            );
        }

        let mut decoded_trailing = valid_sixteen.clone();
        decoded_trailing.push(0xff);
        assert!(!looks_like_private_key_bytes(&decoded_trailing));
        assert_private_classification(
            "decoded trailing byte",
            &STANDARD.encode(decoded_trailing),
            false,
        );

        let valid_algorithm = algorithm_identifier(&[]);
        let malformed_der = [
            ("wrong outer tag", {
                let mut value = valid_sixteen.clone();
                value[0] = 0x31;
                value
            }),
            (
                "indefinite outer length",
                vec![0x30, 0x80, 0x02, 0x01, 0, 0, 0],
            ),
            ("truncated length bytes", vec![0x30, 0x82, 0x01]),
            ("overdeclared content", vec![0x30, 0x10, 0x02, 0x01, 0]),
            ("length overflow", vec![0x30, 0x84, 0xff, 0xff, 0xff, 0xff]),
            ("malformed child", vec![0x30, 0x03, 0x02, 0x02, 0]),
            (
                "generic complete DER",
                der_sequence(&[der_tlv(0x04, &[1, 2, 3, 4])]),
            ),
            ("PKCS#8 wrong version", pkcs8_fixture(&[2], 4)),
            (
                "PKCS#8 empty OID",
                der_sequence(&[
                    der_tlv(0x02, &[0]),
                    der_sequence(&[der_tlv(0x06, &[])]),
                    der_tlv(0x04, &[0x5a; 4]),
                ]),
            ),
            (
                "PKCS#8 wrong OID tag",
                der_sequence(&[
                    der_tlv(0x02, &[0]),
                    der_sequence(&[der_tlv(0x05, &[0x2a])]),
                    der_tlv(0x04, &[0x5a; 4]),
                ]),
            ),
            (
                "PKCS#8 missing private child",
                der_sequence(&[der_tlv(0x02, &[0]), valid_algorithm.clone()]),
            ),
            (
                "PKCS#8 wrong private child tag",
                der_sequence(&[
                    der_tlv(0x02, &[0]),
                    valid_algorithm.clone(),
                    der_tlv(0x03, &[0x5a; 4]),
                ]),
            ),
            (
                "PKCS#8 invalid later child",
                der_sequence(&[
                    der_tlv(0x02, &[0]),
                    valid_algorithm.clone(),
                    der_tlv(0x04, &[0x5a; 4]),
                    der_tlv(0x05, &[]),
                ]),
            ),
            (
                "PKCS#8 too many algorithm parameters",
                der_sequence(&[
                    der_tlv(0x02, &[0]),
                    algorithm_identifier(&[der_tlv(0x05, &[]), der_tlv(0x05, &[])]),
                    der_tlv(0x04, &[0x5a; 4]),
                ]),
            ),
        ];
        for (label, fixture) in malformed_der {
            assert!(!looks_like_private_key_bytes(&fixture), "{label}");
            assert_private_classification(label, &STANDARD.encode(fixture), false);
        }

        for fixture in [
            pkcs8_fixture(&[0], 4),
            pkcs8_fixture(&[1], 4),
            der_sequence(&[
                der_tlv(0x02, &[0]),
                algorithm_identifier(&[der_tlv(0x05, &[])]),
                der_tlv(0x04, &[0x5a; 4]),
                der_tlv(0x81, &[1]),
                der_tlv(0xa0, &[2]),
                der_tlv(0xa1, &[3]),
            ]),
            pkcs1_fixture(0),
            pkcs1_fixture(1),
            {
                let mut children = (0..9)
                    .map(|index| der_tlv(0x02, &[usize::from(index != 0) as u8]))
                    .collect::<Vec<_>>();
                children.push(der_sequence(&[]));
                der_sequence(&children)
            },
            sec1_fixture(1),
            der_sequence(&[
                der_tlv(0x02, &[1]),
                der_tlv(0x04, &[0x5a; 16]),
                der_tlv(0xa0, &[1]),
                der_tlv(0xa1, &[2]),
            ]),
        ] {
            assert!(looks_like_private_key_bytes(&fixture));
            assert_private_classification(
                "valid private shape variant",
                &STANDARD.encode(fixture),
                true,
            );
        }

        let mut pkcs1_wrong_tag = (0..9)
            .map(|index| der_tlv(if index == 4 { 0x04 } else { 0x02 }, &[0]))
            .collect::<Vec<_>>();
        let pkcs1_too_few = der_sequence(&pkcs1_wrong_tag[..8]);
        let pkcs1_wrong_integer_tag = der_sequence(&pkcs1_wrong_tag);
        pkcs1_wrong_tag = (0..9)
            .map(|index| der_tlv(0x02, &[usize::from(index != 0) as u8]))
            .collect();
        let mut tenth_wrong = pkcs1_wrong_tag.clone();
        tenth_wrong.push(der_tlv(0x04, &[]));
        let mut eleventh = pkcs1_wrong_tag.clone();
        eleventh.push(der_sequence(&[]));
        eleventh.push(der_tlv(0x02, &[1]));
        for (label, fixture) in [
            ("PKCS#1 too few integers", pkcs1_too_few),
            ("PKCS#1 wrong integer tag", pkcs1_wrong_integer_tag),
            ("PKCS#1 wrong version", pkcs1_fixture(2)),
            ("PKCS#1 wrong tenth child", der_sequence(&tenth_wrong)),
            ("PKCS#1 eleventh child", der_sequence(&eleventh)),
            ("SEC1 version zero", sec1_fixture(0)),
            ("SEC1 version two", sec1_fixture(2)),
            (
                "SEC1 missing private octets",
                der_sequence(&[der_tlv(0x02, &[1])]),
            ),
            (
                "SEC1 wrong second tag",
                der_sequence(&[der_tlv(0x02, &[1]), der_tlv(0x03, &[0x5a; 16])]),
            ),
            (
                "SEC1 invalid later child",
                der_sequence(&[
                    der_tlv(0x02, &[1]),
                    der_tlv(0x04, &[0x5a; 16]),
                    der_tlv(0xa2, &[1]),
                ]),
            ),
        ] {
            assert!(!looks_like_private_key_bytes(&fixture), "{label}");
            assert_private_classification(label, &STANDARD.encode(fixture), false);
        }

        for allowed in [
            ED25519_SPKI_PUBLIC_KEY,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "rl_0123456789abcdef0123456789abcdef",
            "neutral-opaque-identifier",
            "résumé-owner",
            "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFB",
        ] {
            assert_private_classification("explicit non-private control", allowed, false);
        }
    }

    #[test]
    fn sd_n10_false_positive_controls_and_unicode_are_allowed() {
        for allowed in [
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "rl_0123456789abcdef0123456789abcdef",
            "appfw.domain_change@1",
            "QUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFBQUFB",
            "résumé-owner",
            "neutral-opaque-identifier",
        ] {
            assert!(!contains_encoded_private_key(allowed));
        }
    }

    #[test]
    fn sd_n11_structural_helpers_require_exact_private_shapes() {
        assert!(!looks_like_private_key_bytes(&[0x30, 0x03, 0x02, 0x01, 0]));
        assert!(!looks_like_private_key_bytes(&[0x30, 0x80, 0, 0]));
        assert!(!looks_like_private_key_bytes(&[0x30, 0x01, 0x00, 0x00]));
        assert!(!algorithm_identifier_is_well_formed(&[0x06, 0x00]));
    }

    #[test]
    fn sd_n12_input_ceiling_and_resource_nonclaim_are_enforced() {
        let at_limit = format!("{}-----BEGIN PRIVATE KEY-----", "x".repeat(32_741));
        assert_eq!(at_limit.len(), MAX_PRIVATE_KEY_INPUT_BYTES);
        assert!(contains_encoded_private_key(&at_limit));
        let over_limit = format!("{}A", at_limit);
        assert_eq!(over_limit.len(), MAX_PRIVATE_KEY_INPUT_BYTES + 1);
        assert!(!contains_encoded_private_key(&over_limit));
    }
}
