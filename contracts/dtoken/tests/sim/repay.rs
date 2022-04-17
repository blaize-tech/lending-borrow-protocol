use crate::utils::{
    initialize_controller, initialize_dtoken, initialize_utoken, new_user, view_balance, initialize_dtoken_with_custom_interest_rate
};
use near_sdk::json_types::U128;
use near_sdk_sim::{call, init_simulator, view, ContractAccount, UserAccount};

use controller::ActionType::Borrow;
use dtoken::RepayInfo;
use general::Price;

fn repay_no_borrow_fixture() -> (
    ContractAccount<dtoken::ContractContract>,
    ContractAccount<test_utoken::ContractContract>,
    UserAccount,
) {
    let root = init_simulator(None);

    // Initialize
    let user = new_user(&root, "user".parse().unwrap());
    let (uroot, utoken) = initialize_utoken(&root);
    let (_croot, controller) = initialize_controller(&root);
    let (_droot, dtoken) = initialize_dtoken(&root, utoken.account_id(), controller.account_id());

    call!(
        uroot,
        utoken.mint(dtoken.account_id(), U128(0)),
        0,
        100000000000000
    );

    call!(
        uroot,
        utoken.mint(user.account_id(), U128(20)),
        0,
        100000000000000
    );

    (dtoken, utoken, user)
}

fn repay_fixture() -> (
    ContractAccount<dtoken::ContractContract>,
    ContractAccount<controller::ContractContract>,
    ContractAccount<test_utoken::ContractContract>,
    UserAccount,
    UserAccount,
) {
    let root = init_simulator(None);

    let user = new_user(&root, "user".parse().unwrap());
    let (_uroot, utoken) = initialize_utoken(&root);
    let (_croot, controller) = initialize_controller(&root);
    let (_droot, dtoken) = initialize_dtoken_with_custom_interest_rate(
        &root,
        utoken.account_id(),
        controller.account_id(),
    );

    call!(
        utoken.user_account,
        utoken.mint(dtoken.account_id(), U128(100)),
        0,
        100000000000000
    );

    call!(
        utoken.user_account,
        utoken.mint(user.account_id(), U128(800)),
        0,
        100000000000000
    );

    call!(
        controller.user_account,
        controller.upsert_price(
            dtoken.account_id(),
            &Price {
                ticker_id: "weth".to_string(),
                value: U128(20000),
                volatility: U128(100),
                fraction_digits: 4
            }
        ),
        deposit = 0
    )
    .assert_success();

    (dtoken, controller, utoken, user, root)
}

#[test]
fn scenario_repay_no_borrow() {
    let (dtoken, utoken, user) = repay_no_borrow_fixture();

    let action = "\"Repay\"".to_string();

    call!(
        user,
        utoken.ft_transfer_call(
            dtoken.account_id(),
            U128(20),
            Some("REPAY".to_string()),
            action
        ),
        deposit = 1
    )
    .assert_success();

    let user_balance: String = view!(utoken.ft_balance_of(user.account_id())).unwrap_json();
    assert_eq!(
        user_balance,
        20.to_string(),
        "As user has never borrowed, transfer shouldn't be done"
    );
}

#[test]
fn scenario_repay() {
    let (dtoken, controller, utoken, user, root) = repay_fixture();

    let action = "\"Supply\"".to_string();

    call!(
        user,
        utoken.ft_transfer_call(
            dtoken.account_id(),
            U128(30),
            Some("SUPPLY".to_string()),
            action
        ),
        deposit = 1
    )
    .assert_success();

    root.borrow_runtime_mut().produce_blocks(100).unwrap();

    call!(user, dtoken.borrow(U128(5)), deposit = 0).assert_success();

    root.borrow_runtime_mut().produce_blocks(100).unwrap();

    let dtoken_balance: String = view!(utoken.ft_balance_of(dtoken.account_id())).unwrap_json();
    let repay_info = call!(
        user,
        dtoken.view_repay_info(U128(dtoken_balance.parse().unwrap())),
        deposit = 0
    )
    .unwrap_json::<RepayInfo>();

    let repay_amount = u128::from(repay_info.total_amount)
        + u128::from(repay_info.accrued_interest_per_block) * 10;

    let user_balance_before_repay: String =
        view!(utoken.ft_balance_of(user.account_id())).unwrap_json();

    let action = "\"Repay\"".to_string();

    call!(
        user,
        utoken.ft_transfer_call(
            dtoken.account_id(),
            U128(repay_amount),
            Some("REPAY".to_string()),
            action
        ),
        deposit = 1
    )
    .assert_success();

    let user_balance: String = view!(utoken.ft_balance_of(user.account_id())).unwrap_json();
    assert_ne!(user_balance, user_balance_before_repay, "Repay wasn`t done");

    let user_balance: u128 = view!(dtoken.get_account_borrows(user.account_id())).unwrap_json();
    assert_eq!(user_balance, 0, "Borrow balance on dtoken should be 0");

    let user_balance: u128 =
        view_balance(&controller, Borrow, user.account_id(), dtoken.account_id());
    assert_eq!(user_balance, 0, "Borrow balance on controller should be 0");
}
