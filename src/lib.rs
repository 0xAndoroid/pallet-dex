#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::traits::StaticLookup;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod test;

type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::dispatch::HasCompact;
    use frame_support::{pallet_prelude::*, Blake2_128Concat};
    use frame_system::pallet_prelude::*;
    use pallet_multi_token::multi_token::MultiTokenTrait;
    use sp_runtime::traits::{AtLeast32BitUnsigned, Zero};
    use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub};

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type Balance: Member
            + Parameter
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + MaxEncodedLen
            + TypeInfo;

        type AssetId: Member
            + Parameter
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + HasCompact
            + MaybeSerializeDeserialize
            + MaxEncodedLen
            + TypeInfo
            + Zero;

        type Share: Get<u64>
            + TypeInfo
            + Member
            + Parameter
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + MaxEncodedLen;

        #[pallet::constant]
        type DefaultShare: Get<Self::Share>
            + TypeInfo
            + Member
            + Parameter
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + MaxEncodedLen;

        type MultiToken: MultiTokenTrait<Self, Self::AssetId, Self::Balance>;
    }

    #[pallet::storage]
    #[pallet::getter(fn get_pool)]
    pub type Pools<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,                         // Pool address
        (T::AssetId, T::AssetId, T::Balance), // Pair of assets in the pool & Pool constant
    >;

    #[pallet::storage]
    #[pallet::getter(fn get_pool_share)]
    pub type PoolShares<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AccountId, // Pool address
        Blake2_128Concat,
        T::AccountId,
        T::Share, // Pair of assets in the pool & Pool constant
    >;

    #[pallet::storage]
    pub type TotalPoolShares<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::Share>;

    #[pallet::event]
    #[pallet::generate_deposit(pub fn deposit_event)]
    pub enum Event<T: Config> {
        PoolCreated {
            creator: T::AccountId,
            pool_account: T::AccountId,
            first_asset: T::AssetId,
            second_asset: T::AssetId,
        },
        Swapped {
            operator: T::AccountId,
            pool_account: T::AccountId,
            first_asset: T::AssetId,
            first_asset_amount: T::Balance,
            second_asset: T::AssetId,
            second_asset_amount: T::Balance,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        Overflow,
        DepositingZeroAmount,
        PoolAlreadyExists,
        NotEnoughBalance,
        NoSuchTokenInPool,
        EmptyPool,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1000)]
        pub fn init(
            origin: OriginFor<T>,
            pool_address: AccountIdLookupOf<T>,
            first_token_id: T::AssetId,
            first_token_amount: T::Balance,
            second_token_id: T::AssetId,
            second_token_amount: T::Balance,
        ) -> DispatchResult {
            let creator = ensure_signed(origin)?;
            let pool = T::Lookup::lookup(pool_address)?;

            ensure!(
                !first_token_amount.is_zero() && !second_token_amount.is_zero(),
                Error::<T>::DepositingZeroAmount
            );
            ensure!(Self::get_pool(&pool) == None, Error::<T>::PoolAlreadyExists);
            Self::check_balance(&first_token_id, &creator, first_token_amount.clone())?;
            Self::check_balance(&second_token_id, &creator, second_token_amount.clone())?;

            T::MultiToken::safe_transfer(
                creator.clone(),
                creator.clone(),
                pool.clone(),
                first_token_id.clone(),
                first_token_amount.clone(),
            )?;

            T::MultiToken::safe_transfer(
                creator.clone(),
                creator.clone(),
                pool.clone(),
                second_token_id.clone(),
                second_token_amount.clone(),
            )?;

            let pool_constant = first_token_amount
                .checked_mul(&second_token_amount)
                .ok_or(Error::<T>::Overflow)?;

            Pools::<T>::insert(
                &pool,
                (
                    first_token_id.clone(),
                    second_token_id.clone(),
                    pool_constant,
                ),
            );
            PoolShares::<T>::insert(&pool, &creator, T::DefaultShare::get());
            TotalPoolShares::<T>::insert(&pool, T::DefaultShare::get());

            Ok(())
        }

        #[pallet::weight(1000)]
        pub fn swap_token(
            origin: OriginFor<T>,
            pool_address: AccountIdLookupOf<T>,
            token_id: T::AssetId,
            amount: T::Balance,
        ) -> DispatchResult {
            let operator = ensure_signed(origin)?;
            let pool = T::Lookup::lookup(pool_address)?;

            ensure!(!amount.is_zero(), Error::<T>::DepositingZeroAmount);
            ensure!(Self::get_pool(&pool) == None, Error::<T>::PoolAlreadyExists);
            Self::check_balance(&token_id, &operator, amount.clone())?;

            // We have already checked that pool exists
            let (first_asset_id, second_asset_id, constant) = Self::get_pool(&pool).unwrap();
            let corresponding_token_id = if token_id == first_asset_id {
                second_asset_id
            } else if token_id == second_asset_id {
                first_asset_id
            } else {
                ensure!(false, Error::<T>::NoSuchTokenInPool);
                first_asset_id
            };

            T::MultiToken::safe_transfer(
                operator.clone(),
                operator.clone(),
                pool.clone(),
                token_id.clone(),
                amount.clone(),
            )?;

            let pool_origin_token_balance =
                T::MultiToken::get_balance(&token_id, &pool).ok_or(Error::<T>::EmptyPool)?;
            let pool_dest_token_balance =
                T::MultiToken::get_balance(&corresponding_token_id, &pool)
                    .ok_or(Error::<T>::EmptyPool)?;

            let partial_calculation = constant
                .checked_div(
                    &pool_origin_token_balance
                        .checked_add(&amount)
                        .ok_or(Error::<T>::Overflow)?,
                )
                .ok_or(Error::<T>::Overflow)?;
            let swap_token_result = pool_dest_token_balance
                .checked_sub(&partial_calculation)
                .ok_or(Error::<T>::Overflow)?;

            T::MultiToken::safe_transfer(
                pool.clone(),
                pool.clone(),
                operator.clone(),
                corresponding_token_id.clone(),
                swap_token_result.clone(),
            )?;

            Self::deposit_event(Event::<T>::Swapped {
                operator,
                pool_account: pool,
                first_asset: first_asset_id,
                first_asset_amount: amount,
                second_asset: corresponding_token_id,
                second_asset_amount: swap_token_result,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn check_balance(
            id: &T::AssetId,
            account: &T::AccountId,
            needed_balace: T::Balance,
        ) -> Result<(), Error<T>> {
            match T::MultiToken::get_balance(id, account) {
                Some(balance) => {
                    ensure!(needed_balace >= balance, Error::<T>::NotEnoughBalance);
                }
                None => {
                    return Err(Error::<T>::NotEnoughBalance);
                }
            }
            Ok(())
        }
    }
}