use crate::voting::Error;

#[ink::trait_definition]
pub trait Votinground {
    #[ink(message)]
    fn init_round(&mut self) -> Result<(), Error>;

    #[ink(message)]
    fn get_remaining_time(&mut self) -> Result<u64, Error>;

    #[ink(message)]
    fn has_round_expired(&mut self) -> Result<bool, Error>;
}