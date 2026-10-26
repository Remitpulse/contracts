extern crate std;
use super::*;
use ed25519_dalek::{Signer, SigningKey};
use soroban_sdk::{
    auth::ContractContext,
    testutils::{Address as _, Ledger},
    token, vec, IntoVal,
};

fn register_account(e: &Env, config: Config) -> Address {
    #[cfg(feature = "wasm-tests")]
    {
        e.register(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../target/wasm32v1-none/release/remitpulse_account.wasm"
            ))
            .as_slice(),
            (config,),
        )
    }
    #[cfg(not(feature = "wasm-tests"))]
    {
        e.register(Account, (config,))
    }
}

/// A valid uncompressed P-256 point (the generator) used by constructor tests.
fn valid_passkey(e: &Env) -> BytesN<65> {
    use p256::elliptic_curve::sec1::ToEncodedPoint;
    BytesN::from_array(
        e,
        p256::ecdsa::SigningKey::from_bytes((&[3u8; 32]).into())
            .unwrap()
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .try_into()
            .unwrap(),
    )
}

fn valid_config(e: &Env) -> Config {
    Config {
        keys: vec![e, valid_passkey(e)],
        threshold: 1,
        rp_id_hash: e
            .crypto()
            .sha256(&Bytes::from_slice(e, b"remitpulse.example"))
            .into(),
        origin: Bytes::from_slice(e, b"https://remitpulse.example"),
    }
}

fn setup() -> (Env, Address, Address, SigningKey) {
    let e = Env::default();
    let config = valid_config(&e);
    let account = register_account(&e, config);
    let token = e
        .register_stellar_asset_contract_v2(Address::generate(&e))
        .address();
    let signing = SigningKey::from_bytes(&[7; 32]);
    let public = BytesN::from_array(&e, &signing.verifying_key().to_bytes());
    e.mock_all_auths();
    AccountClient::new(&e, &account).add_session(
        &public,
        &Session {
            token: token.clone(),
            recipient: None,
            per_transfer: 10,
            remaining: 15,
            expires_at: 100,
        },
    );
    e.set_auths(&[]);
    (e, account, token, signing)
}
fn check(
    e: &Env,
    account: &Address,
    asset: &Address,
    signing: &SigningKey,
    amount: i128,
) -> Result<(), Error> {
    let payload = BytesN::from_array(e, &[9; 32]);
    let signature = Signature::Session(
        BytesN::from_array(e, &signing.verifying_key().to_bytes()),
        BytesN::from_array(e, &signing.sign(&payload.to_array()).to_bytes()),
    );
    let contexts = vec![
        e,
        Context::Contract(ContractContext {
            contract: asset.clone(),
            fn_name: Symbol::new(e, "transfer"),
            args: (account.clone(), Address::generate(e), amount).into_val(e),
        }),
    ];
    e.try_invoke_contract_check_auth::<Error>(account, &payload, signature.into_val(e), &contexts)
        .map_err(|err| err.expect("contract error"))
}
#[test]
fn session_budget_scope_expiry_and_revocation() {
    let (e, account, asset, signing) = setup();
    assert_eq!(
        check(&e, &account, &Address::generate(&e), &signing, 1),
        Err(Error::Scope)
    );
    assert_eq!(check(&e, &account, &asset, &signing, 11), Err(Error::Limit));
    assert_eq!(check(&e, &account, &asset, &signing, 0), Err(Error::Limit));
    assert_eq!(check(&e, &account, &asset, &signing, -1), Err(Error::Limit));
    assert_eq!(check(&e, &account, &asset, &signing, 10), Ok(()));
    assert_eq!(check(&e, &account, &asset, &signing, 6), Err(Error::Limit));
    e.ledger().with_mut(|l| l.timestamp = 100);
    assert_eq!(
        check(&e, &account, &asset, &signing, 1),
        Err(Error::Expired)
    );
    e.mock_all_auths();
    AccountClient::new(&e, &account)
        .revoke_session(&BytesN::from_array(&e, &signing.verifying_key().to_bytes()));
    e.set_auths(&[]);
    assert_eq!(
        check(&e, &account, &asset, &signing, 1),
        Err(Error::Invalid)
    );
}

/// Integration: real Soroban custom-account auth, signed payload, and SAC token transfer.
#[test]
fn signed_session_transfers_tokens() {
    use soroban_sdk::xdr::WriteXdr;
    use soroban_sdk::{xdr, Val};
    let (e, account, asset, signing) = setup();
    let recipient = Address::generate(&e);
    e.mock_all_auths();
    token::StellarAssetClient::new(&e, &asset).mint(&account, &20);
    e.set_auths(&[]);
    // Build the exact invocation and network-bound authorization payload as a client would.
    let invocation = xdr::SorobanAuthorizedInvocation {
        function: xdr::SorobanAuthorizedFunction::ContractFn(xdr::InvokeContractArgs {
            contract_address: (&asset).into(),
            function_name: "transfer".try_into().unwrap(),
            args: std::vec![
                (&account).into(),
                (&recipient).into(),
                xdr::ScVal::I128(xdr::Int128Parts { hi: 0, lo: 5 })
            ]
            .try_into()
            .unwrap(),
        }),
        sub_invocations: std::vec![].try_into().unwrap(),
    };
    let expiration = 100;
    let preimage =
        xdr::HashIdPreimage::SorobanAuthorization(xdr::HashIdPreimageSorobanAuthorization {
            network_id: xdr::Hash(e.ledger().network_id().to_array()),
            nonce: 1,
            signature_expiration_ledger: expiration,
            invocation: invocation.clone(),
        });
    let payload = e.crypto().sha256(&Bytes::from_slice(
        &e,
        &preimage.to_xdr(xdr::Limits::none()).unwrap(),
    ));
    let signature = Signature::Session(
        BytesN::from_array(&e, &signing.verifying_key().to_bytes()),
        BytesN::from_array(&e, &signing.sign(&payload.to_array()).to_bytes()),
    );
    let signature_val: Val = signature.into_val(&e);
    e.set_auths(&[xdr::SorobanAuthorizationEntry {
        credentials: xdr::SorobanCredentials::Address(xdr::SorobanAddressCredentials {
            address: (&account).into(),
            nonce: 1,
            signature_expiration_ledger: expiration,
            signature: xdr::ScVal::try_from_val(&e, &signature_val).unwrap(),
        }),
        root_invocation: invocation,
    }]);
    token::Client::new(&e, &asset).transfer(&account, &recipient, &5);
    assert_eq!(token::Client::new(&e, &asset).balance(&recipient), 5);
    assert_eq!(
        AccountClient::new(&e, &account)
            .session(&BytesN::from_array(&e, &signing.verifying_key().to_bytes()))
            .unwrap()
            .remaining,
        10
    );
    // Identical nonce cannot be reused, even with the same signed invocation.
    assert!(token::Client::new(&e, &asset)
        .try_transfer(&account, &recipient, &5)
        .is_err());
}

#[test]
fn passkey_quorum_rejects_missing_and_duplicate_signers() {
    use p256::ecdsa::{signature::Signer as _, Signature as P256Signature, SigningKey as P256Key};
    let e = Env::default();
    let key1 = P256Key::from_bytes((&[3u8; 32]).into()).unwrap();
    let key2 = P256Key::from_bytes((&[4u8; 32]).into()).unwrap();
    let keys = vec![
        &e,
        BytesN::from_array(
            &e,
            key1.verifying_key()
                .to_encoded_point(false)
                .as_bytes()
                .try_into()
                .unwrap(),
        ),
        BytesN::from_array(
            &e,
            key2.verifying_key()
                .to_encoded_point(false)
                .as_bytes()
                .try_into()
                .unwrap(),
        ),
    ];
    let rp: BytesN<32> = e
        .crypto()
        .sha256(&Bytes::from_slice(&e, b"remitpulse.example"))
        .into();
    let origin = Bytes::from_slice(&e, b"https://remitpulse.example");
    let account = register_account(
        &e,
        Config {
            keys,
            threshold: 2,
            rp_id_hash: rp.clone(),
            origin,
        },
    );
    let payload = BytesN::from_array(&e, &[0; 32]);
    let mut assertions = Vec::new(&e);
    for (index, key) in [(0, key1), (1, key2)] {
        let mut auth = Bytes::from(rp.clone());
        auth.append(&Bytes::from_slice(&e, &[5, 0, 0, 0, 0]));
        let json = Bytes::from_slice(&e, br#"{"type":"webauthn.get","challenge":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","origin":"https://remitpulse.example","crossOrigin":false}"#);
        let mut data = auth.clone();
        data.append(&Bytes::from(e.crypto().sha256(&json)));
        let mut bytes = std::vec![0;data.len() as usize];
        data.copy_into_slice(&mut bytes);
        let sig: P256Signature = key.sign(&bytes);
        let sig = sig.normalize_s().unwrap_or(sig);
        assertions.push_back(Assertion {
            key_index: index,
            authenticator_data: auth,
            client_data_json: json,
            signature: BytesN::from_array(&e, &sig.to_bytes().into()),
        });
    }
    let check = |a: Vec<Assertion>| {
        e.try_invoke_contract_check_auth::<Error>(
            &account,
            &payload,
            Signature::Passkeys(a).into_val(&e),
            &vec![&e],
        )
    };
    assert_eq!(
        check(vec![&e, assertions.get(0).unwrap()]),
        Err(Ok(Error::Invalid))
    );
    assert_eq!(
        check(vec![
            &e,
            assertions.get(0).unwrap(),
            assertions.get(0).unwrap()
        ]),
        Err(Ok(Error::Invalid))
    );
    assert_eq!(check(assertions), Ok(()));
    assert!(AccountClient::new(&e, &account)
        .try_add_session(
            &BytesN::from_array(&e, &[7; 32]),
            &Session {
                token: Address::generate(&e),
                recipient: None,
                per_transfer: 1,
                remaining: 2,
                expires_at: 100
            }
        )
        .is_err());
}

#[test]
fn sessions_reject_admin_calls_and_aggregate_overspending() {
    let (e, account, asset, signing) = setup();
    let payload = BytesN::from_array(&e, &[9; 32]);
    let signature = Signature::Session(
        BytesN::from_array(&e, &signing.verifying_key().to_bytes()),
        BytesN::from_array(&e, &signing.sign(&payload.to_array()).to_bytes()),
    );
    let context = Context::Contract(ContractContext {
        contract: asset,
        fn_name: Symbol::new(&e, "transfer"),
        args: (account.clone(), Address::generate(&e), 10i128).into_val(&e),
    });
    let check = |contexts| {
        e.try_invoke_contract_check_auth::<Error>(
            &account,
            &payload,
            signature.clone().into_val(&e),
            &contexts,
        )
    };
    assert_eq!(
        check(vec![&e, context.clone(), context]),
        Err(Ok(Error::Limit))
    );
    assert_eq!(check(vec![&e]), Err(Ok(Error::Scope)));
    let admin = Context::Contract(ContractContext {
        contract: account.clone(),
        fn_name: Symbol::new(&e, "add_session"),
        args: vec![&e],
    });
    assert_eq!(check(vec![&e, admin]), Err(Ok(Error::Scope)));
}

#[test]
fn recipient_binding_and_emergency_pause() {
    let (e, account, asset, signing) = setup();
    let public = BytesN::from_array(&e, &signing.verifying_key().to_bytes());
    let beneficiary = Address::generate(&e);
    let client = AccountClient::new(&e, &account);
    let mut session = client.session(&public).unwrap();
    session.recipient = Some(beneficiary.clone());
    e.mock_all_auths();
    client.add_session(&public, &session);
    e.set_auths(&[]);
    // The existing helper chooses a different recipient for every authorization.
    assert_eq!(check(&e, &account, &asset, &signing, 1), Err(Error::Scope));
    assert_eq!(client.session(&public).unwrap().remaining, 15);
    let payload = BytesN::from_array(&e, &[9; 32]);
    let signature = Signature::Session(
        public.clone(),
        BytesN::from_array(&e, &signing.sign(&payload.to_array()).to_bytes()),
    );
    let contexts = vec![
        &e,
        Context::Contract(ContractContext {
            contract: asset,
            fn_name: Symbol::new(&e, "transfer"),
            args: (account.clone(), beneficiary, 5i128).into_val(&e),
        }),
    ];
    let authorize = || {
        e.try_invoke_contract_check_auth::<Error>(
            &account,
            &payload,
            signature.clone().into_val(&e),
            &contexts,
        )
    };
    assert_eq!(authorize(), Ok(()));
    assert_eq!(client.session(&public).unwrap().remaining, 10);
    assert!(!client.sessions_paused());
    assert!(client.try_set_sessions_paused(&true).is_err());
    e.mock_all_auths();
    client.set_sessions_paused(&true);
    e.set_auths(&[]);
    assert!(client.sessions_paused());
    assert_eq!(authorize(), Err(Ok(Error::Paused)));
    assert_eq!(client.session(&public).unwrap().remaining, 10);
    assert!(client.try_set_sessions_paused(&false).is_err());
    e.mock_all_auths();
    client.set_sessions_paused(&false);
    e.set_auths(&[]);
    assert_eq!(authorize(), Ok(()));
    assert_eq!(client.session(&public).unwrap().remaining, 5);
    assert_eq!(client.config().threshold, 1);
}

#[test]
fn constructor_accepts_valid_config() {
    let e = Env::default();
    let account = e.register(Account, (valid_config(&e),));
    assert_eq!(AccountClient::new(&e, &account).config().threshold, 1);
}

#[test]
fn constructor_rejects_malformed_passkeys() {
    let e = Env::default();
    let mut short = valid_config(&e);
    short.keys = vec![&e, BytesN::from_array(&e, &[4; 64])];
    assert!(e.try_register(Account, (&short,)).is_err());

    let mut bad_prefix = valid_config(&e);
    let mut bytes = [4u8; 65];
    bytes[0] = 0x03;
    bad_prefix.keys = vec![&e, BytesN::from_array(&e, &bytes)];
    assert!(e.try_register(Account, (&bad_prefix,)).is_err());

    // Coordinates equal to the P-256 field prime are not on the curve.
    let mut out_of_field = valid_config(&e);
    let mut bytes = [0u8; 65];
    bytes[0] = 4;
    let prime = [
        0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff,
    ];
    bytes[1..33].copy_from_slice(&prime);
    bytes[33..65].copy_from_slice(&prime);
    out_of_field.keys = vec![&e, BytesN::from_array(&e, &bytes)];
    assert!(e.try_register(Account, (&out_of_field,)).is_err());
}

#[test]
fn constructor_rejects_malformed_origin() {
    let e = Env::default();
    for origin in [
        &b"http://remitpulse.example"[..],
        b"https://",
        b"https://remitpulse.example/",
        b"https://remit pulse.example",
        b"https://remitpulse.example?x=1",
        b"https://remitpulse.example#frag",
    ] {
        let mut config = valid_config(&e);
        config.origin = Bytes::from_slice(&e, origin);
        assert!(e.try_register(Account, (&config,)).is_err());
    }
}

#[test]
fn failed_authorization_keeps_budget_unchanged() {
    let (e, account, asset, signing) = setup();
    let public = BytesN::from_array(&e, &signing.verifying_key().to_bytes());
    let payload = BytesN::from_array(&e, &[9; 32]);
    let forged = Signature::Session(public.clone(), BytesN::from_array(&e, &[0; 64]));
    let contexts = vec![
        &e,
        Context::Contract(ContractContext {
            contract: asset,
            fn_name: Symbol::new(&e, "transfer"),
            args: (account.clone(), Address::generate(&e), 5i128).into_val(&e),
        }),
    ];
    assert!(e
        .try_invoke_contract_check_auth::<Error>(&account, &payload, forged.into_val(&e), &contexts)
        .is_err());
    assert_eq!(
        AccountClient::new(&e, &account)
            .session(&public)
            .unwrap()
            .remaining,
        15
    );
}
