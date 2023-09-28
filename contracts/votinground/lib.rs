//use ink_lang as ink;
#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use self::votinground::VotingRoundRef;

#[cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
#[ink::contract]
mod votinground {
    #[ink(storage)]
    pub struct VotingRound {
        start_time: Timestamp,
        duration: Timestamp,
    }

    // #[ink(event)]
    // pub struct RoundExpired {
    //     round_number: u64,
    // }

    impl VotingRound {
        #[ink(constructor)]
        pub fn new(duration: Timestamp) -> Self {
            let start_time = Self::env().block_timestamp();
            Self { start_time, duration }
        }

        #[ink(message)]
        pub fn has_round_expired(&self) -> bool {
            let current_time = Self::env().block_timestamp();
            current_time >= self.start_time + self.duration
        }

        #[ink(message)]
        pub fn get_remaining_time(&self) -> Timestamp {
            let current_time = Self::env().block_timestamp();
            if current_time >= self.start_time + self.duration {
                0
            } else {
                (self.start_time + self.duration) - current_time
            }
        }
    }
}
