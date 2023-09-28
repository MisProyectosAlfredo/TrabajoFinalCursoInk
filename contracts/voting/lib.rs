#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub mod votingtraits;
pub mod votinground;

#[cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
#[ink::contract]
mod voting {
    use ink::{prelude::vec::Vec, storage::Mapping};//, primitives::AccountId};//, primitives::AccountId};//, primitives::AccountId};
    use psp34::psp34::ContractRef;
    use crate::votingtraits::Votingtraits;
    use scale::{Decode, Encode};
    use crate::votinground::Votinground;

    #[ink(event)]
    pub struct NewVoter {
        #[ink(topic)]
        voter_id: AccountId,
    }

    #[ink(event)]
    pub struct RemoveVoter {
        #[ink(topic)]
        voter_id: AccountId,
        #[ink(topic)]
        total_power: i32,
    }

    #[ink(event)]
    pub struct Vote {
        #[ink(topic)]
        total_power: i32,
        #[ink(topic)]
        total_votes: u32,
        #[ink(topic)]
        votation: TypeVote,  
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        value: Balance,
    }

    #[derive(Debug)]
    #[ink::storage_item]
    pub struct Admin {
        address: AccountId,
        modified_date: u64,
    }

    /// Error management.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotIsAdmin,
        MustBeItSelf,
        VoterAlreadyExists,
        VoterNotExist,
        NotVoteItSelf,
        NotIsVoter,
        NftNotMint,
        RoundNotStarted,
        RoundStarted,
        NoEqualReputation,
        FundsAreNotEnough,
    }

    /// Definition type of vote.
    #[derive(PartialEq, Debug, Eq, Clone, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum TypeVote {
        Like,
        Unlike,
    }

    /// Up and down voter votes
    pub type Upvotes = i32;
    pub type Downvotes = i32;

    /// Voter information
    #[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    pub struct Voter {
        pub upvotes: Upvotes,
        pub downvotes: Downvotes,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct Votermirror {
        voter_id: AccountId,
        reputation: i32,
    }

    #[ink(storage)]
    pub struct Voting {
        admin: Admin,
        enabled_voters: Mapping<AccountId, Voter>,
        mirror: Vec<Votermirror>,
        balances: Mapping<AccountId, Balance>,
        total_votes: u32,
        total_power: i32,
        contract: ContractRef,
        start_time: Timestamp,
        duration: Timestamp,
        roundstarted: bool,
    }

    impl Voting {
        #[ink(constructor)]
        pub fn new(admin: AccountId, contract_code_hash: Hash) -> Self {
            let now = Self::env().block_timestamp();
            Self {
                admin: Admin {
                    address: admin,
                    modified_date: now,
                },
                enabled_voters: Mapping::default(),
                mirror: Vec::new(),
                balances: Mapping::default(),
                total_votes: 0,
                total_power: 0,
                contract: ContractRef::new()
                    .code_hash(contract_code_hash)
                    .endowment(0)
                    .salt_bytes(Vec::new()) // Sequence of bytes
                    .instantiate(),
                start_time: Self::env().block_timestamp(),
                duration: 60000,
                roundstarted: false,
            }
        }

        #[ink(message)]
        pub fn add_voter(&mut self, voter_id: AccountId) -> Result<(), Error> {
            if self.env().caller() != self.admin.address {
                return Err(Error::NotIsAdmin);
            }
            if self.enabled_voters.contains(voter_id) {
                return Err(Error::VoterAlreadyExists);
            }

            self.enabled_voters.insert(voter_id, &Voter{upvotes: 0, downvotes: 0});
            self.mirror.push( Votermirror {
                voter_id,
                reputation: 0,
            });
            self.env().emit_event(NewVoter { voter_id });
            Ok(())
        }

        #[ink(message)]
        pub fn remove_voter(&mut self, voter_id: AccountId) -> Result<(), Error> {
            if self.env().caller() != self.admin.address {
                return Err(Error::NotIsAdmin);
            }
            if !self.enabled_voters.contains(voter_id) {
                return Err(Error::VoterNotExist);
            }
            if self.env().caller() == voter_id {
                return Err(Error::NotVoteItSelf);
            }

            if self.helper_get_reputation_mapping(voter_id) != self.helper_get_reputation_vec(voter_id) {
                return Err(Error::NoEqualReputation);
            }

            self.total_power -= self.helper_get_reputation_mapping(voter_id);

            self.enabled_voters.remove(voter_id);

            let index = self.get_index(voter_id);
            self.mirror.remove(index.unwrap());            

            self.env().emit_event(RemoveVoter { voter_id, total_power: self.total_power });
            Ok(())
        }

        #[ink(message)]
        pub fn vote(&mut self, voter_id: AccountId, value: TypeVote) -> Result<(), Error> {
            if !self.enabled_voters.contains(self.env().caller()) {
                return Err(Error::NotIsVoter);
            }
            if !self.enabled_voters.contains(voter_id) {
                return Err(Error::VoterNotExist);
            }
            if self.env().caller() == voter_id {
                return Err(Error::NotVoteItSelf);
            }

            let caller = self.env().caller();

            if let Some(voter) = self.enabled_voters.get(caller) {
                let up_caller = voter.upvotes;
                let down_caller = voter.downvotes;
                let reputation_caller = up_caller + down_caller;

                let power = self.power_of_vote(reputation_caller);
                let power_positive = 2 * power;

                if let Some(candidate) = self.enabled_voters.get(voter_id) {
                    let up_candidate = candidate.upvotes;
                    let down_candidate = candidate.downvotes;

                    let index = self.get_index(voter_id);
                    let mirror_voter = self.mirror.get_mut(index.unwrap()).unwrap();
                    
                    if value == TypeVote::Like {
                        self.enabled_voters.insert(voter_id, &Voter{upvotes: up_candidate + power_positive, downvotes: down_candidate});
                        mirror_voter.reputation = up_candidate + power_positive + down_candidate;
                        self.total_power += power_positive;
                    } else {
                        self.enabled_voters.insert(voter_id, &Voter{upvotes: up_candidate, downvotes: down_candidate + power});
                        mirror_voter.reputation = up_candidate + down_candidate + power;
                        self.total_power += power;
                    }
                }
            }

            self.total_votes += 1;
            
            let resultmint = self.contract.mint_token(self.env().caller());
            if resultmint.is_err() {
                return Err(Error::NftNotMint);
            }

            self.env().emit_event(Vote { total_power: self.total_power, total_votes: self.total_votes, votation: value});
            Ok(())
        }

        #[ink(message)]
        pub fn get_reputation(&self, voter_id: AccountId) -> Result<i32, Error> {
            if self.env().caller() != voter_id {
                return Err(Error::MustBeItSelf);
            }
            if !self.enabled_voters.contains(voter_id) {
                return Err(Error::VoterNotExist);
            }
            Ok(self.helper_get_reputation_mapping(voter_id))
        }

        #[ink(message)]
        pub fn get_reputation_mirror(&self, voter_id: AccountId) -> Result<i32, Error> {      
            let index = self.get_index(voter_id);

            if index.is_none() {
                return Err(Error::VoterNotExist);
            }
            Ok(self.helper_get_reputation_vec(voter_id))
        }

        #[ink(message)]
        pub fn get_balance_nft(&self, voter_id: AccountId) -> Result<u32, Error> {
            if self.env().caller() != voter_id {
                return Err(Error::MustBeItSelf);
            }
            if !self.enabled_voters.contains(voter_id) {
                return Err(Error::VoterNotExist);
            }
            Ok(self.contract.balance(voter_id))            
        }

        #[ink(message)]
        pub fn init_round(&mut self) -> Result<(), Error> {
            if self.roundstarted {
                return Err(Error::RoundStarted);
            }

            self.roundstarted = true;
            self.start_time = Self::env().block_timestamp();
            Ok(())
        }

        #[ink(message)]
        pub fn get_remaining_time(&mut self) -> Result<u64, Error> {
            if !self.roundstarted {
                return Err(Error::RoundNotStarted);
            }

            let current_time = Self::env().block_timestamp();
            if current_time >= self.start_time + self.duration {
                Ok(0)
            } else {
                Ok((self.start_time + self.duration) - current_time)
            }
        }

        #[ink(message)]
        pub fn has_round_expired(&mut self) -> Result<bool, Error> {
            if !self.roundstarted {
                return Err(Error::RoundNotStarted);
            }

            let current_time = Self::env().block_timestamp();         
            Ok(current_time >= self.start_time + self.duration)
        }

        #[ink(message)]
        pub fn ranking(&mut self) -> Result<i32, Error> {
            self.sorted_for_reputation();
            Ok(self.mirror[0].reputation)
        }

        #[ink(message)]
        pub fn get_balance_admin(&self) ->  Result<Balance, Error> {
            if self.env().caller() != self.admin.address {
                return Err(Error::NotIsAdmin);
            }
            Ok(self.env().balance())
        }

        #[ink(message)]
        pub fn get_balance(&self, voter_id: AccountId) ->  Result<Balance, Error> {
            Ok(self.helper_get_balance(voter_id))
        }

        #[ink(message)]
        pub fn transfer_admin_to(&mut self, to: AccountId, value: Balance) -> Result<(), Error> {
            let balance_from = self.env().balance();          

            if balance_from < value {
                return Err(Error::FundsAreNotEnough);
            }

            let balance_to = self.helper_get_balance(to);
            self.balances.insert(to, &(balance_to + value));

            self.env().emit_event(Transfer {from: Some(self.env().caller()), to, value});
            Ok(())
        }

        fn power_of_vote(&mut self, reputation: i32) -> i32 {
            if self.total_power == 0 {
                1
            } else {
                let power = (reputation * 100)/self.total_power;
                match power {
                    0 => 1,
                    1...33 => 10,
                    34...66 => 20,
                    _ => 30
                }
            }
        }
        
        fn get_index(&self, voter_id: AccountId) -> Option<usize> {
            self.mirror
                .iter()
                .position(|c| c.voter_id == voter_id)
        }

        fn helper_get_reputation_mapping(&self, voter_id: AccountId) -> i32 {
            let mut reputation = 0;
            if let Some(voter) = self.enabled_voters.get(voter_id) {
                let up = voter.upvotes;
                let down = voter.downvotes;
                reputation = up + down;
            }
            reputation
        }

        fn helper_get_reputation_vec(&self, voter_id: AccountId) -> i32 {
            let index = self.get_index(voter_id);
            let voter_mirror = self.mirror.get(index.unwrap()).unwrap();
            voter_mirror.reputation
        }

        fn sorted_for_reputation(&mut self) {
             self.mirror.sort_by(|a, b| b.reputation.cmp(&a.reputation));            
        }

        fn helper_get_balance(&self, voter_id: AccountId) -> Balance {
            self.balances.get(voter_id).unwrap_or(0)
        }
    }

    impl Votingtraits for Voting {    
        #[ink(message)]
        fn vote(&mut self, voter_id: AccountId, value: TypeVote) -> Result<(), Error> {        
            self.vote(voter_id, value).unwrap();
            Ok(())
        }

        #[ink(message)]
        fn get_reputation(&self, voter_id: AccountId) -> Result<i32, Error> {            
            Ok(self.get_reputation(voter_id).unwrap())
        }
    }

    impl Votinground for Voting {
        #[ink(message)]
        fn init_round(&mut self) -> Result<(), Error> {
            self.init_round().unwrap();
            Ok(())
        }
        
        #[ink(message)]
        fn get_remaining_time(&mut self) -> Result<u64, Error> {
            self.get_remaining_time()
        }
        
        #[ink(message)]
        fn has_round_expired(&mut self) -> Result<bool, Error> {
            self.has_round_expired()
        }        
    }
}