use crate::{mock::*, Error};

use frame_benchmarking::frame_support::assert_noop;
use frame_support::assert_ok;
use pallet_multi_token::multi_token::MultiTokenTrait;

#[test]
fn init_pool() {
    new_test_ext().execute_with(|| {
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 0, 100));
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 1, 100));
        assert_ok!(Dex::init(Origin::signed(1), 314159265, 0, 50, 1, 50));

        assert_eq!(Dex::get_pool(314159265), Some((0, 1, 2500)));
        assert_eq!(Dex::get_pool_share(314159265, 1), Some(10000));
    });
}

#[test]
fn swap_tokens() {
    new_test_ext().execute_with(|| {
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 0, 100));
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 1, 100));
        assert_ok!(MultiTokenPallet::transfer(Origin::signed(1), 1, 2, 0, 10));
        assert_ok!(Dex::init(Origin::signed(1), 314159265, 0, 50, 1, 50));
        assert_ok!(Dex::swap_token(Origin::signed(2), 314159265, 0, 10));

        assert_eq!(MultiTokenPallet::get_balance(&0, &2), Some(0));
        assert_eq!(MultiTokenPallet::get_balance(&1, &2), Some(9));
    });
}

#[test]
fn depositing_liquidity() {
    new_test_ext().execute_with(|| {
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 0, 100));
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 1, 100));
        assert_ok!(MultiTokenPallet::transfer(Origin::signed(1), 1, 2, 0, 10));
        assert_ok!(MultiTokenPallet::transfer(Origin::signed(1), 1, 2, 1, 10));
        assert_ok!(Dex::init(Origin::signed(1), 314159265, 0, 50, 1, 50));
        assert_ok!(Dex::deposit(Origin::signed(2), 314159265, 0, 10));

        assert_eq!(MultiTokenPallet::get_balance(&0, &2), Some(0));
        assert_eq!(MultiTokenPallet::get_balance(&1, &2), Some(0));
        assert_eq!(Dex::get_pool_share(314159265, 2), Some(2000));
        assert_eq!(Dex::get_total_pool_shares(314159265), Some(12000));
    });
}

#[test]
fn withdrawing_liquidity() {
    new_test_ext().execute_with(|| {
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 0, 100));
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 1, 100));
        assert_ok!(Dex::init(Origin::signed(1), 314159265, 0, 50, 1, 50));
        assert_ok!(Dex::withdraw(Origin::signed(1), 314159265, 0, 10));

        assert_eq!(MultiTokenPallet::get_balance(&0, &1), Some(60));
        assert_eq!(MultiTokenPallet::get_balance(&1, &1), Some(60));
        assert_eq!(Dex::get_pool_share(314159265, 1), Some(8000));
        assert_eq!(Dex::get_total_pool_shares(314159265), Some(8000));

        assert_ok!(Dex::withdraw(Origin::signed(1), 314159265, 0, 40));

        assert_eq!(MultiTokenPallet::get_balance(&0, &1), Some(100));
        assert_eq!(MultiTokenPallet::get_balance(&1, &1), Some(100));
        assert_eq!(Dex::get_pool_share(314159265, 1), Some(0));
        assert_eq!(Dex::get_total_pool_shares(314159265), Some(0));
    });
}

#[test]
#[should_panic]
fn init_pool_with_same_assets() {
    new_test_ext().execute_with(|| {
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 0, 100));
        assert_ok!(Dex::init(Origin::signed(1), 314159265, 0, 50, 0, 50));
    });
}

#[test]
fn abuse_without_tokens() {
    new_test_ext().execute_with(|| {
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 0, 11000));
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 1, 10100));
        assert_ok!(MultiTokenPallet::transfer(Origin::signed(1), 1, 2, 0, 10000));
        assert_ok!(MultiTokenPallet::transfer(Origin::signed(1), 1, 2, 1, 10000));
        assert_noop!(Dex::init(Origin::signed(1), 314159265, 0, 500, 1, 500), Error::<Test>::NotEnoughBalance);
        assert_ok!(Dex::init(Origin::signed(1), 314159265, 0, 50, 1, 50));
        assert_noop!(Dex::swap_token(Origin::signed(1), 314159265, 1, 500), Error::<Test>::NotEnoughBalance);
        assert_noop!(Dex::deposit(Origin::signed(1), 314159265, 1, 500), Error::<Test>::NotEnoughBalance);
        assert_ok!(Dex::deposit(Origin::signed(2), 314159265, 0, 10000));
        assert_noop!(Dex::withdraw(Origin::signed(1), 314159265, 1, 500), Error::<Test>::Overflow);
        assert_noop!(Dex::withdraw(Origin::signed(1), 314159265, 1, 51), Error::<Test>::Overflow);
        assert_ok!(Dex::withdraw(Origin::signed(1), 314159265, 1, 50));
    });
}

#[test]
fn using_uninitialized_pool() {
    new_test_ext().execute_with(|| {
        assert_noop!(Dex::swap_token(Origin::signed(2), 314159265, 0, 10), Error::<Test>::NoSuchPool);
    });
}

#[test]
fn zero_amounts() {
    new_test_ext().execute_with(|| {
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 0, 100));
        assert_ok!(MultiTokenPallet::create(Origin::signed(1)));
        assert_ok!(MultiTokenPallet::mint(Origin::signed(1), 1, 100));
        assert_noop!(Dex::init(Origin::signed(1), 314159265, 0, 0, 1, 50), Error::<Test>::DepositingZeroAmount);
        assert_ok!(Dex::init(Origin::signed(1), 314159265, 0, 50, 1, 50));
        assert_noop!(Dex::swap_token(Origin::signed(1), 314159265, 1, 0), Error::<Test>::DepositingZeroAmount);
        assert_noop!(Dex::deposit(Origin::signed(1), 314159265, 1, 0), Error::<Test>::DepositingZeroAmount);
        assert_noop!(Dex::withdraw(Origin::signed(1), 314159265, 1, 0), Error::<Test>::WithdrawingZeroAmount);
    });
}