// This is an adapted copy of lockup contract
// https://github.com/near/core-contracts/tree/master/lockup
// used to reason what is happening with lockup and vesting
use serde::{Deserialize, Serialize};
use borsh::{self, BorshDeserialize, BorshSerialize};
pub use near_sdk::json_types::{Base64VecU8, U128, U64};
use near_sdk::{env, AccountId, Balance};

use uint::construct_uint;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

/// Raw type for duration in nanoseconds
pub type Duration = u64;
/// Raw type for timestamp in nanoseconds
pub type Timestamp = u64;

/// Timestamp in nanosecond wrapped into a struct for JSON serialization as a string.
pub type WrappedTimestamp = U64;
/// Duration in nanosecond wrapped into a struct for JSON serialization as a string.
pub type WrappedDuration = U64;
/// Balance wrapped into a struct for JSON serialization as a string.
pub type WrappedBalance = U128;

/// Hash of Vesting schedule.
pub type Hash = Vec<u8>;

/// The result of the transfer poll.
/// Contains The timestamp when the proposal was voted in.
pub type PollResult = Option<WrappedTimestamp>;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct LockupContract {
    /// The account ID of the owner.
    pub owner_account_id: AccountId,

    /// Information about lockup schedule and the amount.
    pub lockup_information: LockupInformation,

    /// Information about vesting including schedule or termination status.
    pub vesting_information: VestingInformation,

    /// Account ID of the staking pool whitelist contract.
    pub staking_pool_whitelist_account_id: AccountId,

    /// Information about staking and delegation.
    /// `Some` means the staking information is available and the staking pool contract is selected.
    /// `None` means there is no staking pool selected.
    pub staking_information: Option<StakingInformation>,

    /// The account ID that the NEAR Foundation, that has the ability to terminate vesting.
    pub foundation_account_id: Option<AccountId>,
}

/// Contains information about token lockups.
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct LockupInformation {
    /// The amount in yocto-NEAR tokens locked for this account.
    pub lockup_amount: Balance,
    /// The amount of tokens that were withdrawn by NEAR foundation due to early termination
    /// of vesting.
    /// This amount has to be accounted separately from the lockup_amount to make sure
    /// linear release is not being affected.
    pub termination_withdrawn_tokens: Balance,
    /// The lockup duration in nanoseconds from the moment when transfers are enabled to unlock the
    /// lockup amount of tokens.
    pub lockup_duration: Duration,
    /// If present, the duration when the full lockup amount will be available. The tokens are
    /// linearly released from the moment transfers are enabled.
    pub release_duration: Option<Duration>,
    /// The optional absolute lockup timestamp in nanoseconds which locks the tokens until this
    /// timestamp passes.
    pub lockup_timestamp: Option<Timestamp>,
    /// The information to indicate when the lockup period starts.
    pub transfers_information: TransfersInformation,
}

/// Contains information about the transfers. Whether transfers are enabled or disabled.
#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum TransfersInformation {
    /// The timestamp when the transfers were enabled. The lockup period starts at this timestamp.
    TransfersEnabled {
        transfers_timestamp: WrappedTimestamp,
    },
    /// The account ID of the transfers poll contract, to check if the transfers are enabled.
    /// The lockup period will start when the transfer voted to be enabled.
    /// At the launch of the network transfers are disabled for all lockup contracts, once transfers
    /// are enabled, they can't be disabled and don't need to be checked again.
    TransfersDisabled { transfer_poll_account_id: AccountId },
}

/// Describes the status of transactions with the staking pool contract or terminated unvesting
/// amount withdrawal.
#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum TransactionStatus {
    /// There are no transactions in progress.
    Idle,
    /// There is a transaction in progress.
    Busy,
}

/// Contains information about current stake and delegation.
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct StakingInformation {
    /// The Account ID of the staking pool contract.
    pub staking_pool_account_id: AccountId,

    /// Contains status whether there is a transaction in progress.
    pub status: TransactionStatus,

    /// The amount of tokens that were deposited from this account to the staking pool.
    /// NOTE: The unstaked amount on the staking pool might be higher due to staking rewards.
    pub deposit_amount: WrappedBalance,
}

/// Contains information about vesting schedule.
#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Clone, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct VestingSchedule {
    /// The timestamp in nanosecond when the vesting starts. E.g. the start date of employment.
    pub start_timestamp: WrappedTimestamp,
    /// The timestamp in nanosecond when the first part of lockup tokens becomes vested.
    /// The remaining tokens will vest continuously until they are fully vested.
    /// Example: a 1 year of employment at which moment the 1/4 of tokens become vested.
    pub cliff_timestamp: WrappedTimestamp,
    /// The timestamp in nanosecond when the vesting ends.
    pub end_timestamp: WrappedTimestamp,
}

impl VestingSchedule {
    pub fn assert_valid(&self) {
        assert!(
            self.start_timestamp.0 <= self.cliff_timestamp.0,
            "Cliff timestamp can't be earlier than vesting start timestamp"
        );
        assert!(
            self.cliff_timestamp.0 <= self.end_timestamp.0,
            "Cliff timestamp can't be later than vesting end timestamp"
        );
        assert!(
            self.start_timestamp.0 < self.end_timestamp.0,
            "The total vesting time should be positive"
        );
    }
}

/// Initialization argument type to define the vesting schedule
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum VestingScheduleOrHash {
    /// The vesting schedule is private and this is a hash of (vesting_schedule, salt).
    /// In JSON, the hash has to be encoded with base64 to a string.
    VestingHash(Base64VecU8),
    /// The vesting schedule (public)
    VestingSchedule(VestingSchedule),
}

/// Contains information about vesting that contains vesting schedule and termination information.
#[derive(Serialize, BorshDeserialize, BorshSerialize, PartialEq, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum VestingInformation {
    None,
    /// Vesting schedule is hashed for privacy and only will be revealed if the NEAR foundation
    /// has to terminate vesting.
    /// The contract assume the vesting schedule doesn't affect lockup release and duration, because
    /// the vesting started before transfers were enabled and the duration is shorter or the same.
    VestingHash(Base64VecU8),
    /// Explicit vesting schedule.
    VestingSchedule(VestingSchedule),
    /// The information about the early termination of the vesting schedule.
    /// It means the termination of the vesting is currently in progress.
    /// Once the unvested amount is transferred out, `VestingInformation` is removed.
    Terminating(TerminationInformation),
}

/// Describes the status of transactions with the staking pool contract or terminated unvesting
/// amount withdrawal.
#[derive(
    BorshDeserialize, BorshSerialize, Deserialize, Serialize, PartialEq, Copy, Clone, Debug,
)]
#[serde(crate = "near_sdk::serde")]
pub enum TerminationStatus {
    /// Initial stage of the termination in case there are deficit on the account.
    VestingTerminatedWithDeficit,
    /// A transaction to unstake everything is in progress.
    UnstakingInProgress,
    /// The transaction to unstake everything from the staking pool has completed.
    EverythingUnstaked,
    /// A transaction to withdraw everything from the staking pool is in progress.
    WithdrawingFromStakingPoolInProgress,
    /// Everything is withdrawn from the staking pool. Ready to withdraw out of the account.
    ReadyToWithdraw,
    /// A transaction to withdraw tokens from the account is in progress.
    WithdrawingFromAccountInProgress,
}

/// Contains information about early termination of the vesting schedule.
#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, PartialEq, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct TerminationInformation {
    /// The amount of tokens that are unvested and has to be transferred back to NEAR Foundation.
    /// These tokens are effectively locked and can't be transferred out and can't be restaked.
    pub unvested_amount: WrappedBalance,

    /// The status of the withdrawal. When the unvested amount is in progress of withdrawal the
    /// status will be marked as busy, to avoid withdrawing the funds twice.
    pub status: TerminationStatus,
}

/// Contains a vesting schedule with a salt.
#[derive(BorshSerialize, Deserialize, Serialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct VestingScheduleWithSalt {
    /// The vesting schedule
    pub vesting_schedule: VestingSchedule,
    /// Salt to make the hash unique
    pub salt: Base64VecU8,
}

impl VestingScheduleWithSalt {
    pub fn hash(&self) -> Hash {
        env::sha256(&self.try_to_vec().expect("Failed to serialize"))
    }
}

impl LockupContract {
    /// Get the amount of tokens that are locked in this account due to lockup or vesting.
    pub fn get_locked_amount(&self, timestamp: u64) -> WrappedBalance {
        let lockup_amount = self.lockup_information.lockup_amount;
        if let TransfersInformation::TransfersEnabled {
            transfers_timestamp,
        } = &self.lockup_information.transfers_information
        {
            let lockup_timestamp = std::cmp::max(
                transfers_timestamp
                    .0
                    .saturating_add(self.lockup_information.lockup_duration),
                self.lockup_information.lockup_timestamp.unwrap_or(0),
            );
            let block_timestamp = timestamp;
            if lockup_timestamp <= block_timestamp {
                let unreleased_amount =
                    if let &Some(release_duration) = &self.lockup_information.release_duration {
                        let end_timestamp = lockup_timestamp.saturating_add(release_duration);
                        if block_timestamp >= end_timestamp {
                            // Everything is released
                            0
                        } else {
                            let time_left = U256::from(end_timestamp - block_timestamp);
                            let unreleased_amount = U256::from(lockup_amount) * time_left
                                / U256::from(release_duration);
                            // The unreleased amount can't be larger than lockup_amount because the
                            // time_left is smaller than total_time.
                            unreleased_amount.as_u128()
                        }
                    } else {
                        0
                    };

                let unvested_amount = match &self.vesting_information {
                    VestingInformation::VestingSchedule(vs) => self.get_unvested_amount(vs.clone(), block_timestamp),
                    VestingInformation::Terminating(terminating) => terminating.unvested_amount,
                    // Vesting is private, so we can assume the vesting started before lockup date.
                    _ => U128(0),
                };
                return std::cmp::max(
                    unreleased_amount
                        .saturating_sub(self.lockup_information.termination_withdrawn_tokens),
                    unvested_amount.0,
                )
                .into();
            }
        }
        // The entire balance is still locked before the lockup timestamp.
        (lockup_amount - self.lockup_information.termination_withdrawn_tokens).into()
    }

    /// Get the amount of tokens that are locked in this account due to vesting or release schedule.
    /// Takes raw vesting schedule, in case the internal vesting schedule is private.
    pub fn get_unvested_amount(&self, vesting_schedule: VestingSchedule, block_timestamp: u64) -> WrappedBalance {
        let lockup_amount = self.lockup_information.lockup_amount;
        match &self.vesting_information {
            VestingInformation::Terminating(termination_information) => {
                termination_information.unvested_amount
            }
            VestingInformation::None => U128::from(0),
            _ => {
                if block_timestamp < vesting_schedule.cliff_timestamp.0 {
                    // Before the cliff, nothing is vested
                    lockup_amount.into()
                } else if block_timestamp >= vesting_schedule.end_timestamp.0 {
                    // After the end, everything is vested
                    0.into()
                } else {
                    // cannot overflow since block_timestamp < vesting_schedule.end_timestamp
                    let time_left = U256::from(vesting_schedule.end_timestamp.0 - block_timestamp);
                    // The total time is positive. Checked at the contract initialization.
                    let total_time = U256::from(
                        vesting_schedule.end_timestamp.0 - vesting_schedule.start_timestamp.0,
                    );
                    let unvested_amount = U256::from(lockup_amount) * time_left / total_time;
                    // The unvested amount can't be larger than lockup_amount because the
                    // time_left is smaller than total_time.
                    unvested_amount.as_u128().into()
                }
            }
        }
    }
}
