//! RemitPulse standalone WebAuthn verification contract.
#![no_std]
pub use remitpulse_webauthn_verifier::{verify_assertion, Assertion};
use soroban_sdk::{contract, contractimpl, Bytes, BytesN, Env};

#[contract]
pub struct WebAuthn;
#[contractimpl]
impl WebAuthn {
    pub fn verify(
        e: Env,
        key: BytesN<65>,
        rp_id_hash: BytesN<32>,
        origin: Bytes,
        payload: BytesN<32>,
        assertion: Assertion,
    ) {
        verify_assertion(&e, &key, &rp_id_hash, &origin, &payload, &assertion);
    }
}

#[cfg(test)]
mod test {
    extern crate std;
    use super::*;
    use p256::ecdsa::{signature::Signer, Signature, SigningKey};

    pub fn fixture(
        e: &Env,
        flags: u8,
        origin: &str,
        challenge: &str,
    ) -> (BytesN<65>, BytesN<32>, BytesN<32>, Assertion) {
        let signing = SigningKey::from_bytes((&[3u8; 32]).into()).unwrap();
        let point = signing.verifying_key().to_encoded_point(false);
        let key = BytesN::from_array(e, point.as_bytes().try_into().unwrap());
        let rp: BytesN<32> = e
            .crypto()
            .sha256(&Bytes::from_slice(e, b"remitpulse.example"))
            .into();
        let payload = BytesN::from_array(e, &[0; 32]);
        let mut auth = Bytes::from(rp.clone());
        auth.push_back(flags);
        auth.append(&Bytes::from_slice(e, &[0; 4]));
        let json = std::format!(
            r#"{{"type":"webauthn.get","challenge":"{challenge}","origin":"{origin}","crossOrigin":false}}"#
        );
        let json = Bytes::from_slice(e, json.as_bytes());
        let mut data = auth.clone();
        data.append(&Bytes::from(e.crypto().sha256(&json)));
        let mut raw = std::vec![0;data.len() as usize];
        data.copy_into_slice(&mut raw);
        let sig: Signature = signing.sign(&raw);
        let sig = sig.normalize_s().unwrap_or(sig);
        (
            key,
            rp,
            payload,
            Assertion {
                key_index: 0,
                authenticator_data: auth,
                client_data_json: json,
                signature: BytesN::from_array(e, &sig.to_bytes().into()),
            },
        )
    }
    #[test]
    fn valid_assertion() {
        let e = Env::default();
        let challenge = std::str::from_utf8(&[b'A'; 43]).unwrap();
        let (key, rp, payload, a) = fixture(&e, 5, "https://remitpulse.example", challenge);
        let id = e.register(WebAuthn, ());
        WebAuthnClient::new(&e, &id).verify(
            &key,
            &rp,
            &Bytes::from_slice(&e, b"https://remitpulse.example"),
            &payload,
            &a,
        );
    }
    #[test]
    fn rejects_invalid_binding_and_flags() {
        let e = Env::default();
        let id = e.register(WebAuthn, ());
        let client = WebAuthnClient::new(&e, &id);
        let origin = Bytes::from_slice(&e, b"https://remitpulse.example");
        for (flags, site, challenge) in [
            (
                1,
                "https://remitpulse.example",
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            ),
            (
                5,
                "https://evil.example",
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            ),
            (5, "https://remitpulse.example", "wrong"),
        ] {
            let (key, rp, payload, a) = fixture(&e, flags, site, challenge);
            assert!(client.try_verify(&key, &rp, &origin, &payload, &a).is_err());
        }
        let (key, rp, payload, mut a) = fixture(
            &e,
            5,
            "https://remitpulse.example",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        );
        a.signature = BytesN::from_array(&e, &[0; 64]);
        assert!(client.try_verify(&key, &rp, &origin, &payload, &a).is_err());
    }
    #[test]
    fn rejects_ambiguous_client_json_and_wrong_rp() {
        let e = Env::default();
        let id = e.register(WebAuthn, ());
        let client = WebAuthnClient::new(&e, &id);
        let origin = Bytes::from_slice(&e, b"https://remitpulse.example");
        let (key, rp, payload, assertion) = fixture(
            &e,
            5,
            "https://remitpulse.example",
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        );
        assert!(client
            .try_verify(
                &key,
                &BytesN::from_array(&e, &[0; 32]),
                &origin,
                &payload,
                &assertion
            )
            .is_err());
        for json in [
            r#"{"type":"webauthn.get","type":"webauthn.create","challenge":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","origin":"https://remitpulse.example"}"#,
            r#"{"type":"webauthn.get","challenge":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","origin":"https://remitpulse.example","crossOrigin":true}"#,
        ] {
            let mut a = assertion.clone();
            a.client_data_json = Bytes::from_slice(&e, json.as_bytes());
            // Re-sign malformed client data so failure cannot be explained by a stale signature.
            let mut signed = a.authenticator_data.clone();
            signed.append(&Bytes::from(e.crypto().sha256(&a.client_data_json)));
            let mut raw = std::vec![0; signed.len() as usize];
            signed.copy_into_slice(&mut raw);
            let sig: Signature = SigningKey::from_bytes((&[3; 32]).into())
                .unwrap()
                .sign(&raw);
            a.signature =
                BytesN::from_array(&e, &sig.normalize_s().unwrap_or(sig).to_bytes().into());
            assert!(client.try_verify(&key, &rp, &origin, &payload, &a).is_err());
        }
    }
}
