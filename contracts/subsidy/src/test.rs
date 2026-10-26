use super::*;
use soroban_sdk::testutils::Address as _;
#[test]
fn reimburses_once_and_requires_admin() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let vault = e.register(Subsidy, (&admin,));
    let recipient = Address::generate(&e);
    let asset = e.register_stellar_asset_contract_v2(admin).address();
    e.mock_all_auths();
    token::StellarAssetClient::new(&e, &asset).mint(&vault, &100);
    let client = SubsidyClient::new(&e, &vault);
    let id = BytesN::from_array(&e, &[1; 32]);
    client.reimburse(&id, &asset, &recipient, &10);
    assert_eq!(token::Client::new(&e, &asset).balance(&recipient), 10);
    assert!(client.try_reimburse(&id, &asset, &recipient, &10).is_err());
    e.set_auths(&[]);
    assert!(client
        .try_reimburse(&BytesN::from_array(&e, &[2; 32]), &asset, &recipient, &10)
        .is_err());
}

#[test]
fn failed_transfer_rolls_back_reimbursement_and_can_be_retried() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let vault = e.register(Subsidy, (&admin,));
    let asset = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let recipient = Address::generate(&e);
    let client = SubsidyClient::new(&e, &vault);
    let id = BytesN::from_array(&e, &[3; 32]);
    e.mock_all_auths();
    assert_eq!(client.admin(), admin);
    assert_eq!(
        client.try_reimburse(&id, &asset, &recipient, &0),
        Err(Ok(Error::InvalidAmount))
    );
    assert!(client.try_reimburse(&id, &asset, &recipient, &10).is_err());
    assert!(!client.is_reimbursed(&id));
    token::StellarAssetClient::new(&e, &asset).mint(&vault, &10);
    client.reimburse(&id, &asset, &recipient, &10);
    assert!(client.is_reimbursed(&id));
    assert_eq!(token::Client::new(&e, &asset).balance(&recipient), 10);
    assert_eq!(
        client.try_reimburse(&id, &asset, &recipient, &10),
        Err(Ok(Error::AlreadyReimbursed))
    );
}

#[test]
fn replay_rejection_is_scoped_to_active_epoch() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let vault = e.register(Subsidy, (&admin,));
    let asset = e.register_stellar_asset_contract_v2(admin.clone()).address();
    let recipient = Address::generate(&e);
    let client = SubsidyClient::new(&e, &vault);
    let id = BytesN::from_array(&e, &[4; 32]);
    e.mock_all_auths();
    token::StellarAssetClient::new(&e, &asset).mint(&vault, &100);

    // The active epoch starts at 0.
    assert_eq!(client.active_epoch(), 0);
    assert!(!client.is_reimbursed(&id));

    // Within the same epoch, replay is rejected.
    client.reimburse(&id, &asset, &recipient, &10);
    assert!(client.is_reimbursed(&id));
    assert_eq!(
        client.try_reimburse(&id, &asset, &recipient, &10),
        Err(Ok(Error::AlreadyReimbursed))
    );

    // Rotation requires admin auth.
    e.set_auths(&[]);
    assert!(client.try_rotate_epoch().is_err());
    e.mock_all_auths();

    // Rotation increments the active epoch.
    client.rotate_epoch();
    assert_eq!(client.active_epoch(), 1);

    // After rotation the marker is no longer in the active epoch, so the
    // same ID can be reimbursed again.
    assert!(!client.is_reimbursed(&id));
    client.reimburse(&id, &asset, &recipient, &10);
    assert!(client.is_reimbursed(&id));
    assert_eq!(token::Client::new(&e, &asset).balance(&recipient), 20);
    assert_eq!(
        client.try_reimburse(&id, &asset, &recipient, &10),
        Err(Ok(Error::AlreadyReimbursed))
    );
}

#[test]
fn failed_transfer_rolls_back_marker_within_epoch() {
    let e = Env::default();
    let admin = Address::generate(&e);
    let vault = e.register(Subsidy, (&admin,));
    let asset = e.register_stellar_asset_contract_v2(admin.clone()).address();
    let recipient = Address::generate(&e);
    let client = SubsidyClient::new(&e, &vault);
    let id = BytesN::from_array(&e, &[5; 32]);
    e.mock_all_auths();
    assert_eq!(client.active_epoch(), 0);
    // No funds: transfer fails and the marker must not persist.
    assert!(client.try_reimburse(&id, &asset, &recipient, &10).is_err());
    assert!(!client.is_reimbursed(&id));
    token::StellarAssetClient::new(&e, &asset).mint(&vault, &10);
    client.reimburse(&id, &asset, &recipient, &10);
    assert!(client.is_reimbursed(&id));
    assert_eq!(
        client.try_reimburse(&id, &asset, &recipient, &10),
        Err(Ok(Error::AlreadyReimbursed))
    );
}
