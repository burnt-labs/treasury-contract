use crate::contract::{execute, instantiate};
use crate::error::ContractError;
use crate::execute::{
    accept_admin, cancel_proposed_admin, deploy_fee_grant, init, migrate as migrate_exec,
    propose_admin, remove_grant_config, revoke_allowance, update_fee_config, update_grant_config,
    update_params, validate_params, withdraw_coins,
};
use crate::grant::allowance::format_allowance;
use crate::grant::{Any, FeeConfig, GrantConfig};
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::query;
use crate::state::Params;
use cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin;
use cosmos_sdk_proto::cosmos::feegrant::v1beta1::{
    AllowedMsgAllowance, BasicAllowance, PeriodicAllowance,
};
use cosmos_sdk_proto::prost::Message;
use cosmos_sdk_proto::traits::MessageExt;
use cosmos_sdk_proto::xion::v1::{AuthzAllowance, ContractsAllowance, MultiAnyAllowance};
use cosmos_sdk_proto::Timestamp;
use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi};
use cosmwasm_std::{Addr, Binary, CosmosMsg, DepsMut, Env, MessageInfo};

const SEND_TYPE_URL: &str = "/cosmos.bank.v1beta1.MsgSend";
const BASIC_ALLOWANCE_URL: &str = "/cosmos.feegrant.v1beta1.BasicAllowance";

// GrantConfig and FeeConfig keep `description` private to the grant module, so
// tests build them through their derived Deserialize rather than a struct
// literal.
fn make_grant_config(authorization: Any, optional: bool) -> GrantConfig {
    serde_json::from_value(serde_json::json!({
        "description": "test grant config",
        "authorization": serde_json::to_value(&authorization).unwrap(),
        "optional": optional,
    }))
    .unwrap()
}

fn make_fee_config(allowance: Option<Any>, expiration: Option<u32>) -> FeeConfig {
    serde_json::from_value(serde_json::json!({
        "description": "test fee config",
        "allowance": allowance.map(|a| serde_json::to_value(&a).unwrap()),
        "expiration": expiration,
    }))
    .unwrap()
}

fn basic_allowance_any(amount: &str) -> Any {
    let allowance = BasicAllowance {
        spend_limit: vec![ProtoCoin {
            denom: "uxion".to_string(),
            amount: amount.to_string(),
        }],
        expiration: None,
    };
    Any {
        type_url: BASIC_ALLOWANCE_URL.to_string(),
        value: Binary::new(allowance.to_bytes().unwrap()),
    }
}

fn valid_params() -> Params {
    Params {
        redirect_url: "https://example.com/redirect".to_string(),
        icon_url: "https://example.com/icon.png".to_string(),
        metadata: r#"{"name":"test treasury"}"#.to_string(),
    }
}

fn addr(api: &MockApi, name: &str) -> Addr {
    api.addr_make(name)
}

/// Instantiate a treasury with `admin` in control, one required grant config
/// for MsgSend, and a 1000uxion basic allowance.
fn setup(deps: DepsMut, admin: &Addr) {
    let info = message_info(admin, &[]);
    init(
        deps,
        info,
        Some(admin.clone()),
        vec![SEND_TYPE_URL.to_string()],
        vec![make_grant_config(basic_allowance_any("1000"), false)],
        make_fee_config(Some(basic_allowance_any("1000")), Some(3600)),
        valid_params(),
    )
    .unwrap();
}

fn env_info(api: &MockApi, sender: &str) -> (Env, MessageInfo) {
    (mock_env(), message_info(&addr(api, sender), &[]))
}

// ---------------------------------------------------------------- params ---

#[test]
fn validate_params_accepts_well_formed_input() {
    validate_params(&valid_params()).unwrap();
}

#[test]
fn validate_params_rejects_a_malformed_redirect_url() {
    let params = Params {
        redirect_url: "not a url".to_string(),
        ..valid_params()
    };
    assert!(matches!(
        validate_params(&params),
        Err(ContractError::URLParse(_))
    ));
}

#[test]
fn validate_params_rejects_a_malformed_icon_url() {
    let params = Params {
        icon_url: "://missing-scheme".to_string(),
        ..valid_params()
    };
    assert!(matches!(
        validate_params(&params),
        Err(ContractError::URLParse(_))
    ));
}

#[test]
fn validate_params_rejects_metadata_that_is_not_json() {
    let params = Params {
        metadata: "{not json".to_string(),
        ..valid_params()
    };
    assert!(matches!(
        validate_params(&params),
        Err(ContractError::JsonError(_))
    ));
}

// ----------------------------------------------------------- instantiate ---

#[test]
fn instantiate_stores_the_full_configuration() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    setup(deps.as_mut(), &admin);

    assert_eq!(query::admin(&deps.storage).unwrap(), admin);
    assert_eq!(
        query::grant_config_type_urls(&deps.storage).unwrap(),
        vec![SEND_TYPE_URL.to_string()]
    );
    assert_eq!(query::params(&deps.storage).unwrap(), valid_params());
    assert!(query::fee_config(&deps.storage)
        .unwrap()
        .allowance
        .is_some());
}

#[test]
fn instantiate_rejects_a_missing_admin() {
    let mut deps = mock_dependencies();
    let sender = addr(&deps.api, "sender");
    let msg = InstantiateMsg {
        admin: None,
        type_urls: vec![],
        grant_configs: vec![],
        fee_config: make_fee_config(None, None),
        params: valid_params(),
    };
    let err = instantiate(deps.as_mut(), mock_env(), message_info(&sender, &[]), msg).unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

#[test]
fn instantiate_rejects_mismatched_type_urls_and_grant_configs() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let err = init(
        deps.as_mut(),
        message_info(&admin, &[]),
        Some(admin.clone()),
        vec![SEND_TYPE_URL.to_string(), "/other.Msg".to_string()],
        vec![make_grant_config(basic_allowance_any("1000"), false)],
        make_fee_config(None, None),
        valid_params(),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::ConfigurationMismatch));
}

// ----------------------------------------------------- admin transfer ------

#[test]
fn admin_transfer_requires_the_proposed_admin_to_accept() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let next = addr(&deps.api, "next-admin");
    setup(deps.as_mut(), &admin);

    propose_admin(deps.as_mut(), message_info(&admin, &[]), next.to_string()).unwrap();

    // Still the original admin until the proposal is accepted.
    assert_eq!(query::admin(&deps.storage).unwrap(), admin);
    assert_eq!(query::pending_admin(&deps.storage).unwrap(), next);

    accept_admin(deps.as_mut(), message_info(&next, &[])).unwrap();

    assert_eq!(query::admin(&deps.storage).unwrap(), next);
    assert!(query::pending_admin(&deps.storage).is_err());
}

#[test]
fn propose_admin_rejects_a_non_admin() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let stranger = addr(&deps.api, "stranger");
    setup(deps.as_mut(), &admin);

    let err = propose_admin(
        deps.as_mut(),
        message_info(&stranger, &[]),
        stranger.to_string(),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

#[test]
fn accept_admin_rejects_anyone_but_the_proposed_admin() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let next = addr(&deps.api, "next-admin");
    let stranger = addr(&deps.api, "stranger");
    setup(deps.as_mut(), &admin);

    propose_admin(deps.as_mut(), message_info(&admin, &[]), next.to_string()).unwrap();

    let err = accept_admin(deps.as_mut(), message_info(&stranger, &[])).unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
    assert_eq!(query::admin(&deps.storage).unwrap(), admin);
}

#[test]
fn cancel_proposed_admin_clears_the_proposal() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let next = addr(&deps.api, "next-admin");
    setup(deps.as_mut(), &admin);

    propose_admin(deps.as_mut(), message_info(&admin, &[]), next.to_string()).unwrap();
    cancel_proposed_admin(deps.as_mut(), message_info(&admin, &[])).unwrap();

    assert!(query::pending_admin(&deps.storage).is_err());

    // The cancelled proposal can no longer be accepted.
    let err = accept_admin(deps.as_mut(), message_info(&next, &[])).unwrap_err();
    assert!(matches!(err, ContractError::Std(_)));
}

#[test]
fn cancel_proposed_admin_rejects_a_non_admin() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let next = addr(&deps.api, "next-admin");
    let stranger = addr(&deps.api, "stranger");
    setup(deps.as_mut(), &admin);

    propose_admin(deps.as_mut(), message_info(&admin, &[]), next.to_string()).unwrap();

    let err = cancel_proposed_admin(deps.as_mut(), message_info(&stranger, &[])).unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
    assert_eq!(query::pending_admin(&deps.storage).unwrap(), next);
}

// ------------------------------------------------------- grant configs -----

#[test]
fn update_grant_config_reports_whether_it_overwrote() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    setup(deps.as_mut(), &admin);

    let res = update_grant_config(
        deps.as_mut(),
        message_info(&admin, &[]),
        "/other.Msg".to_string(),
        make_grant_config(basic_allowance_any("5"), true),
    )
    .unwrap();
    assert!(res.events[0]
        .attributes
        .iter()
        .any(|a| a.key == "overwritten" && a.value == "false"));

    let res = update_grant_config(
        deps.as_mut(),
        message_info(&admin, &[]),
        "/other.Msg".to_string(),
        make_grant_config(basic_allowance_any("7"), true),
    )
    .unwrap();
    assert!(res.events[0]
        .attributes
        .iter()
        .any(|a| a.key == "overwritten" && a.value == "true"));
}

#[test]
fn update_grant_config_rejects_a_non_admin() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let stranger = addr(&deps.api, "stranger");
    setup(deps.as_mut(), &admin);

    let err = update_grant_config(
        deps.as_mut(),
        message_info(&stranger, &[]),
        SEND_TYPE_URL.to_string(),
        make_grant_config(basic_allowance_any("1"), true),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

#[test]
fn remove_grant_config_removes_an_existing_entry() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    setup(deps.as_mut(), &admin);

    remove_grant_config(
        deps.as_mut(),
        message_info(&admin, &[]),
        SEND_TYPE_URL.to_string(),
    )
    .unwrap();

    assert!(query::grant_config_type_urls(&deps.storage)
        .unwrap()
        .is_empty());
}

#[test]
fn remove_grant_config_rejects_an_unknown_type_url() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    setup(deps.as_mut(), &admin);

    let err = remove_grant_config(
        deps.as_mut(),
        message_info(&admin, &[]),
        "/never.Registered".to_string(),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::GrantConfigNotFound { .. }));
}

#[test]
fn remove_grant_config_rejects_a_non_admin() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let stranger = addr(&deps.api, "stranger");
    setup(deps.as_mut(), &admin);

    let err = remove_grant_config(
        deps.as_mut(),
        message_info(&stranger, &[]),
        SEND_TYPE_URL.to_string(),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

// --------------------------------------------- fee config, params, funds ---

#[test]
fn update_fee_config_rejects_a_non_admin() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let stranger = addr(&deps.api, "stranger");
    setup(deps.as_mut(), &admin);

    let err = update_fee_config(
        deps.as_mut(),
        message_info(&stranger, &[]),
        make_fee_config(None, None),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

#[test]
fn update_params_rejects_a_non_admin() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let stranger = addr(&deps.api, "stranger");
    setup(deps.as_mut(), &admin);

    let err =
        update_params(deps.as_mut(), message_info(&stranger, &[]), valid_params()).unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

#[test]
fn update_params_validates_before_storing() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    setup(deps.as_mut(), &admin);

    let bad = Params {
        redirect_url: "not a url".to_string(),
        ..valid_params()
    };
    assert!(update_params(deps.as_mut(), message_info(&admin, &[]), bad).is_err());

    // The stored params are untouched by the rejected update.
    assert_eq!(query::params(&deps.storage).unwrap(), valid_params());
}

#[test]
fn withdraw_rejects_a_non_admin() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let stranger = addr(&deps.api, "stranger");
    setup(deps.as_mut(), &admin);

    let err = withdraw_coins(deps.as_mut(), message_info(&stranger, &[]), vec![]).unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

#[test]
fn withdraw_sends_coins_to_the_admin() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    setup(deps.as_mut(), &admin);

    let coins = vec![cosmwasm_std::coin(500, "uxion")];
    let res = withdraw_coins(deps.as_mut(), message_info(&admin, &[]), coins.clone()).unwrap();

    assert_eq!(res.messages.len(), 1);
    match &res.messages[0].msg {
        CosmosMsg::Bank(cosmwasm_std::BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, admin.as_str());
            assert_eq!(amount, &coins);
        }
        other => panic!("expected a bank send, got {other:?}"),
    }
}

// ---------------------------------------------------------- allowances -----

#[test]
fn revoke_allowance_rejects_a_non_admin() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let stranger = addr(&deps.api, "stranger");
    setup(deps.as_mut(), &admin);

    let err = revoke_allowance(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        stranger.clone(),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

#[test]
fn revoke_allowance_emits_a_feegrant_revoke() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let grantee = addr(&deps.api, "grantee");
    setup(deps.as_mut(), &admin);

    let res = revoke_allowance(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        grantee.clone(),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    match &res.messages[0].msg {
        CosmosMsg::Any(any) => {
            assert_eq!(any.type_url, "/cosmos.feegrant.v1beta1.MsgRevokeAllowance");
        }
        other => panic!("expected an Any message, got {other:?}"),
    }
}

/// Regression test for the fix restricting `deploy_fee_grant` to the granter.
///
/// Before that change any caller could trigger — or refresh by
/// revoke-and-reissue — the fee grant of an existing granter/grantee pair
/// without the granter's involvement.
#[test]
fn deploy_fee_grant_rejects_a_caller_who_is_not_the_granter() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let granter = addr(&deps.api, "granter");
    let grantee = addr(&deps.api, "grantee");
    let stranger = addr(&deps.api, "stranger");
    setup(deps.as_mut(), &admin);

    let err = deploy_fee_grant(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        granter.clone(),
        grantee.clone(),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

/// The admin is not exempt: authority over the treasury does not make the
/// admin the granter of somebody else's authz grant.
#[test]
fn deploy_fee_grant_rejects_the_admin_acting_for_another_granter() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let granter = addr(&deps.api, "granter");
    let grantee = addr(&deps.api, "grantee");
    setup(deps.as_mut(), &admin);

    let err = deploy_fee_grant(
        deps.as_mut(),
        mock_env(),
        message_info(&admin, &[]),
        granter,
        grantee,
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

#[test]
fn deploy_fee_grant_routes_the_sender_check_through_execute() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let granter = addr(&deps.api, "granter");
    let grantee = addr(&deps.api, "grantee");
    setup(deps.as_mut(), &admin);

    let (env, info) = env_info(&deps.api, "stranger");
    let err = execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::DeployFeeGrant {
            authz_granter: granter,
            authz_grantee: grantee,
        },
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

// ------------------------------------------------------------- migrate -----

#[test]
fn migrate_rejects_a_non_admin() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    let stranger = addr(&deps.api, "stranger");
    setup(deps.as_mut(), &admin);

    let err = migrate_exec(
        deps.as_mut(),
        mock_env(),
        message_info(&stranger, &[]),
        2,
        Binary::default(),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Unauthorized));
}

#[test]
fn migrate_emits_a_wasm_migrate_for_the_admin() {
    let mut deps = mock_dependencies();
    let admin = addr(&deps.api, "admin");
    setup(deps.as_mut(), &admin);

    let env = mock_env();
    let res = migrate_exec(
        deps.as_mut(),
        env.clone(),
        message_info(&admin, &[]),
        42,
        Binary::default(),
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    match &res.messages[0].msg {
        CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Migrate {
            contract_addr,
            new_code_id,
            ..
        }) => {
            assert_eq!(contract_addr, env.contract.address.as_str());
            assert_eq!(*new_code_id, 42u64);
        }
        other => panic!("expected a wasm migrate, got {other:?}"),
    }
}

// --------------------------------------------------- allowance formatting --

fn grantee_addr() -> Addr {
    MockApi::default().addr_make("grantee")
}

fn granter_addr() -> Addr {
    MockApi::default().addr_make("granter")
}

fn expiry() -> Option<Timestamp> {
    Some(Timestamp {
        seconds: 1_800_000_000,
        nanos: 0,
    })
}

#[test]
fn format_allowance_leaves_a_basic_allowance_alone_without_an_expiration() {
    let input = basic_allowance_any("1000");
    let out = format_allowance(input.clone(), granter_addr(), grantee_addr(), None).unwrap();
    assert_eq!(out, input);
}

#[test]
fn format_allowance_sets_the_expiration_on_a_basic_allowance() {
    let out = format_allowance(
        basic_allowance_any("1000"),
        granter_addr(),
        grantee_addr(),
        expiry(),
    )
    .unwrap();

    let decoded = BasicAllowance::decode(out.value.as_slice()).unwrap();
    assert_eq!(decoded.expiration, expiry());
    assert_eq!(decoded.spend_limit[0].amount, "1000");
}

#[test]
fn format_allowance_sets_the_expiration_inside_a_periodic_allowance() {
    let periodic = PeriodicAllowance {
        basic: Some(BasicAllowance {
            spend_limit: vec![],
            expiration: None,
        }),
        period: None,
        period_spend_limit: vec![],
        period_can_spend: vec![],
        period_reset: None,
    };
    let input = Any {
        type_url: "/cosmos.feegrant.v1beta1.PeriodicAllowance".to_string(),
        value: Binary::new(periodic.to_bytes().unwrap()),
    };

    let out = format_allowance(input, granter_addr(), grantee_addr(), expiry()).unwrap();
    let decoded = PeriodicAllowance::decode(out.value.as_slice()).unwrap();
    assert_eq!(decoded.basic.unwrap().expiration, expiry());
}

#[test]
fn format_allowance_recurses_into_an_allowed_msg_allowance() {
    let allowed = AllowedMsgAllowance {
        allowance: Some(basic_allowance_any("250").into()),
        allowed_messages: vec![SEND_TYPE_URL.to_string()],
    };
    let input = Any {
        type_url: "/cosmos.feegrant.v1beta1.AllowedMsgAllowance".to_string(),
        value: Binary::new(allowed.to_bytes().unwrap()),
    };

    let out = format_allowance(input, granter_addr(), grantee_addr(), expiry()).unwrap();
    let decoded = AllowedMsgAllowance::decode(out.value.as_slice()).unwrap();
    assert_eq!(decoded.allowed_messages, vec![SEND_TYPE_URL.to_string()]);

    let inner = BasicAllowance::decode(decoded.allowance.unwrap().value.as_slice()).unwrap();
    assert_eq!(inner.expiration, expiry());
}

#[test]
fn format_allowance_binds_an_authz_allowance_to_the_grantee() {
    let authz = AuthzAllowance {
        allowance: Some(basic_allowance_any("10").into()),
        authz_grantee: "placeholder-overwritten-by-format".to_string(),
    };
    let input = Any {
        type_url: "/xion.v1.AuthzAllowance".to_string(),
        value: Binary::new(authz.to_bytes().unwrap()),
    };

    let grantee = grantee_addr();
    let out = format_allowance(input, granter_addr(), grantee.clone(), expiry()).unwrap();

    let decoded = AuthzAllowance::decode(out.value.as_slice()).unwrap();
    assert_eq!(decoded.authz_grantee, grantee.to_string());
}

#[test]
fn format_allowance_recurses_into_a_contracts_allowance() {
    let contracts = ContractsAllowance {
        allowance: Some(basic_allowance_any("10").into()),
        contract_addresses: vec!["xion1contract".to_string()],
    };
    let input = Any {
        type_url: "/xion.v1.ContractsAllowance".to_string(),
        value: Binary::new(contracts.to_bytes().unwrap()),
    };

    let out = format_allowance(input, granter_addr(), grantee_addr(), expiry()).unwrap();
    let decoded = ContractsAllowance::decode(out.value.as_slice()).unwrap();
    assert_eq!(
        decoded.contract_addresses,
        vec!["xion1contract".to_string()]
    );

    let inner = BasicAllowance::decode(decoded.allowance.unwrap().value.as_slice()).unwrap();
    assert_eq!(inner.expiration, expiry());
}

#[test]
fn format_allowance_recurses_into_every_arm_of_a_multi_any_allowance() {
    let multi = MultiAnyAllowance {
        allowances: vec![
            basic_allowance_any("1").into(),
            basic_allowance_any("2").into(),
        ],
    };
    let input = Any {
        type_url: "/xion.v1.MultiAnyAllowance".to_string(),
        value: Binary::new(multi.to_bytes().unwrap()),
    };

    let out = format_allowance(input, granter_addr(), grantee_addr(), expiry()).unwrap();
    let decoded = MultiAnyAllowance::decode(out.value.as_slice()).unwrap();
    assert_eq!(decoded.allowances.len(), 2);

    for arm in decoded.allowances {
        let inner = BasicAllowance::decode(arm.value.as_slice()).unwrap();
        assert_eq!(inner.expiration, expiry());
    }
}

#[test]
fn format_allowance_rejects_an_unknown_allowance_type() {
    let input = Any {
        type_url: "/not.A.Real.Allowance".to_string(),
        value: Binary::default(),
    };
    let err = format_allowance(input, granter_addr(), grantee_addr(), expiry()).unwrap_err();
    assert!(matches!(err, ContractError::InvalidAllowanceType { .. }));
}

#[test]
fn format_allowance_rejects_a_wrapper_with_no_inner_allowance() {
    let allowed = AllowedMsgAllowance {
        allowance: None,
        allowed_messages: vec![SEND_TYPE_URL.to_string()],
    };
    let input = Any {
        type_url: "/cosmos.feegrant.v1beta1.AllowedMsgAllowance".to_string(),
        value: Binary::new(allowed.to_bytes().unwrap()),
    };

    let err = format_allowance(input, granter_addr(), grantee_addr(), expiry()).unwrap_err();
    assert!(matches!(err, ContractError::AllowanceUnset));
}
