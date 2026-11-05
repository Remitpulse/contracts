//! RemitPulse WebAuthn ES256 verifier. Challenge = Soroban authorization payload.
#![no_std]
use serde::Deserialize;
use soroban_sdk::{contracttype, Bytes, BytesN, Env};

#[contracttype]
#[derive(Clone)]
pub struct Assertion {
    pub key_index: u32,
    pub authenticator_data: Bytes,
    pub client_data_json: Bytes,
    /// Raw, low-S ECDSA r || s, not the authenticator's ASN.1 DER signature.
    pub signature: BytesN<64>,
}

#[derive(Deserialize)]
struct ClientData<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    challenge: &'a str,
    origin: &'a str,
    #[serde(rename = "crossOrigin", default)]
    cross_origin: bool,
}

pub fn challenge_bytes(payload: &BytesN<32>) -> [u8; 43] {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let input = payload.to_array();
    let mut output = [0u8; 43];
    let mut bits = 0u32;
    let mut count = 0u32;
    let mut j = 0;
    for byte in input {
        bits = (bits << 8) | byte as u32;
        count += 8;
        while count >= 6 {
            count -= 6;
            output[j] = alphabet[((bits >> count) & 63) as usize];
            j += 1;
        }
    }
    output[j] = alphabet[((bits << (6 - count)) & 63) as usize];
    output
}

pub fn verify_assertion(
    e: &Env,
    key: &BytesN<65>,
    rp_id_hash: &BytesN<32>,
    origin: &Bytes,
    payload: &BytesN<32>,
    assertion: &Assertion,
) {
    assert!(
        assertion.authenticator_data.len() >= 37,
        "short authenticator data"
    );
    assert!(
        assertion.authenticator_data.slice(0..32) == Bytes::from(rp_id_hash.clone()),
        "RP ID mismatch"
    );
    // Require both user presence and user verification for privileged operations.
    assert!(
        assertion.authenticator_data.get(32).unwrap() & 5 == 5,
        "UP/UV required"
    );
    assert!(
        assertion.client_data_json.len() <= 2048,
        "client data too large"
    );
    let mut buffer = [0u8; 2048];
    let len = assertion.client_data_json.len() as usize;
    assertion
        .client_data_json
        .copy_into_slice(&mut buffer[..len]);
    let (client, used): (ClientData, usize) =
        serde_json_core::from_slice(&buffer[..len]).expect("invalid client JSON");
    assert!(used == len, "trailing client data");
    assert!(
        client.kind == "webauthn.get" && !client.cross_origin,
        "invalid ceremony"
    );
    assert!(
        Bytes::from_slice(e, client.origin.as_bytes()) == *origin,
        "origin mismatch"
    );
    assert!(
        client.challenge.as_bytes() == challenge_bytes(payload),
        "challenge mismatch"
    );
    let mut signed = assertion.authenticator_data.clone();
    signed.append(&Bytes::from(e.crypto().sha256(&assertion.client_data_json)));
    e.crypto()
        .secp256r1_verify(key, &e.crypto().sha256(&signed), &assertion.signature);
}
