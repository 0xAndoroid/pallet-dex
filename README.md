# pallet-dex
An implementation of AMM decentralized exchange as a pallet for Substrate. Is designed to be used with [pallet-multi-token](https://github.com/AndoroidX/pallet-multi-token).
This pallet uses constant product formula for swaps.

## Config
### `type Balance` 
is a number-like type which is used to store balances, shares of the pool and fees. Assumed to be the same one as the `pallet_multi_token` uses.  
  
### `type AssetId` 
is a number-like type which is used to store id of an asset. Assumed to be the same one as the `pallet_multi_token` uses.  
  
### `type DefaultShare` 
is a default share value that will be assigned to the pool creator on initialization. See [`Shares` section](#shares) of README for explanation.

### `type HundredPercentMinusFee` 
is a number-like constant which stores the value for 100% minus fee. See [`Fee` section](#fees) of README for explanation.

### `type HundredPercent` 
is a number-like constant which stores the value for 100%. See [`Fee` section](#fees) of README for explanation.

## Events
The events are straightforward by their names.
`PoolCreated`, `Swapped`, `Deposited`, `Withdrawed`.

## Errors
```rust
// An arithmetic overflow
Overflow,
// Depositing 0 amount for init, swap or deposit functions
DepositingZeroAmount,
// Trying to withdraw 0 amount from the pool
WithdrawingZeroAmount,
// Trying to initialize pool with an address that already exists
PoolAlreadyExists,
// Trying to interact with a pool that does not exist
NoSuchPool,
// There is not enough balance to perform operation
NotEnoughBalance,
// Trying to deposit/withdraw wrong asset into the pool
NoSuchTokenInPool,
// The pool is dead, no assets in the pool
EmptyPool,
// Initialization of the pool with both assets being same
SameAssetPool,
```

## Storage
### `Pools`
is a map storage, stores info about pool. The key is an `Config::AccountId` of a pool, and value is tuple `(Config::AssetId, Config::AssetId, Config::Balance)` which stores first and second token ids in the pool and pool constant (used in constant product formula) respectively.
### `PoolShares`
is a double map storage, stores pool shares of each user. The keys are `Config::AccountId` - pool address and `Config::AccountId` - user address. Value is a `Config::Balance` - user's share in the pool.
### `TotalPoolShares`
is a map storage, stores sum of all users' shares in the given pool. The key is `Config::AccountId` - pool address, and value is `Config::Balance` sum of all pool shares that users have.

## Pool accounts
Pool account has the same type as user's account has. The pool account is assigned by pool creator in while calling `init` function.

## Shares
This pallet uses shares in order to remember how much of a pool a user owns.  
When a pool is created, a user receives `Config::DefaultShare` share, and this share server as a middle point from now on for this pool. The same value is assigned to `TotalPoolShares` of this pool.  
Whenever someone deposits any liquidity to the pool, their share is calculated based on the total share of the pool and total amount of the assets stored in the pool and is assigned to them in `PoolShares` storage. If user already had a share in the pool, a new share is just added to the previous one.  
*This mechanism allows to lower the amount of reads/writes comparatively to the "storing percentage of the pool on every user" mechanism because we do not need to reassign every user his new share every time someone deposits or withdraws.*

## Fees
Whenever a user swaps some tokens using this pallet, a given fee, declared in config, is kept in order to incentivize liquidity providers.
The fee is declared in config by these two constant types
```rust
type HundredPercent;
type HundredPercentMinusFee;
```
The first type stores a value that would represent a hundred percent, which is a maximum reference point.
The second type stores a value that would represent a hundred percent value minus fee value.  
Let's say we want to enable a 0.3% fee on swaps. In order to do so, we would implement `Config` the following way
```rust
impl pallet_dex::Config for Runtime {
    // snip
    type HundredPercentMinusFee = ConstU128<997>;
    type HundredPercent = ConstU128<1000>;
}
```
So, if we substract `HundredPercent` from `HundredPercentMinusFee` and divide by `HundredPercent`, we would get minus fee. $-0.003$ in the example above.

## Depositing or withdrawing one asset
### Deposits
Depositing one asset in being performed by swapping a portion of this asset into correspondig pool asset and depositing by the regular way. In order to determine how much of an asset we need to swap, the following formula is used  
$p_0=\sqrt{x^2+xt}-x$
where $p_0$ is the amount to be swapped, $x$ is the amount of this asset already in the pool and $t$ is the amount that user willings to deposit.  
### Proof
Assume that user wants to deposit token A only, and there is another token B in the pool.
There is $x$ amount of token A and $y$ amount of token B in the pool before swap. And user has $t$ tokens A that they are willing to deposit. Since they do not want to deposit token B, they have $0$ token B.  
Firstly, we do a swap of the tokens. We swap $p_0$ tokens A for $q_0$ tokens B.
After the swap user would have $t-p_0$ tokens A and $q_0$ tokens B.
And the pool would have $x+p_0$ tokens A and $y-q_0$ tokens B.  
Since we use a constant product AMM, the following statement is true.  
$xy=(x+p_0)(y-q_0)$  
After we deposit all our tokens into the pool, the pool would have 
$x+t$ tokens A and $y$ tokens B.  
Since the ratio of our deposit is the same as ratio of tokens in the pool, the following statement is true.  
${t-p_0 \over q_0} = {x+t \over y}$  
Solving the two systems above, we get our formula for $p_0$ mentioned above.
### Withdrawals
The process of withdrawing only one asset is the reverse of depositing. Firstly we withdraw, and then we swap. The formula for the amount of token A to be withdrawn is similar too $p_0=x-\sqrt{x^2-xt}$. Proof is very similar too.

## Dead pools
The pool is defined as dead when any of these conditions are met  
- <ins>At least one</ins> asset in the pool has a balance of 0
- The total share of the pool is 0  

Whenever user tries to interact with a dead pool (this includes depositing new liquidity into the pool), an `EmptyPool` error would be thrown.

## Possible attacks and drawbacks
### Using a pool address with known private key
Since user defines a pool address on pool creation, it is possible to define an address that the creator knows private key of. Then, when other people give liquidity to the pool, assets can be transferred. Possible solutions are the following
- Generate address at random at the process of pool creation
- Give a community a note to trust only pools that follow a certain pattern. For example, a creator must use pool address that is 20 right most bytes of a sha512 hash of sum of AssetIds. With this pattern it is impossible to get private keys for the address.
- Block sending of assets by signing in the `pallet-multi-token`

### Overflowing
Overflowing is a problem with any computer based mathematics. In this case, we are having a pool constant that shares the same type as token balances, but is a multiplication of two token balances.
With a usage of `u128` for balances, we can use only `u64` for actually storing balances, which makes impossible to use tokens with 18 decimals for swaps.  
The introduction of `u256` might come handy, as Solidity has this type, but it does not seem to appear in Rust any time soon.

### Dead pool with leftover tokens
With the approximations taking place in integer maths, it is possible to simulate a situation in which an existing pool has some balance of token A, but zero balance of token B, making it impossible to get the remaining tokens from the pool. But this amount of tokens in negligible.

### Approximations with depositing or withdrawing one asset
With the introduction of [fees](#fees) into swaps, depositing and withdrawing using only one asset became more inaccurate.  
Depositing wolud make some amount to remain in user's balance and while withdrawing users would receive a little bit less tokens as they have requested. The slippage is approximately equal to the swap fee, so if we set fee to 0, the process would be more percise.  
- It is possible to make swaps from deposit/withdraw function fee excempt, but it is not the best idea since it could result in users abusing this and swapping large volumes without paying fee
- It is also possible to consider fee during deposits/withdrawals, but this would often result in `NotEnoughBalance` error thanks to integer maths approximations.

I decided to ignore this issue, because after all these functions are more of a convinience to a user rather than essential thing. Implementing second solution would make it less convenient.

### Making default pool share
If a default pool share becomes too small, users who deposit small amounts of tokens would have 0 pool share, so it is important to set default pool share to some mid value. For example if `Config::Balance` is `u128`, it would be reasonable to use `u32.MAX` as a default pool share.
But a pool creator can deposit some large amount of tokens, assigning default pool share to this large amount of tokens, and then withdrawing almost all liquidity. This would change default pool share of this pool to a relatively small number that can create issue described above.  
It might be reasonable to note community not to deposit liquidity into 'broken' pool, if one is created. Another good idea is to make default pool share dependent on the amount of tokens that user deposits and leaves in the pool, but this requires more complicated Config.