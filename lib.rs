#![cfg_attr(not(feature = "std"), no_std)]
use ink_lang as ink;

#[ink::contract]
mod erc721 {
    use ink_storage::traits::SpreadAllocate;
    use ink_storage::Mapping;

    use ink_prelude::vec::Vec;
    use scale::{Decode, Encode};

    /// A token ID.
    pub type TokenId = u32;

    #[ink(storage)]
    #[derive(Default, SpreadAllocate)]
    pub struct Erc721 {
        /// Mapping from token to owner.
        token_owner: Mapping<TokenId, AccountId>,
        /// Mapping from owner to all tokens
        owned_tokens: Mapping<AccountId, Vec<TokenId>>,
        /// Mapping from owner to number of owned token.
        owned_tokens_count: Mapping<AccountId, u32>,
        /// Token metadata
        token_data: Mapping<TokenId, NftData>,
        /// All tokens id
        all_tokens: Vec<TokenId>,
        
        /// prices of token
        prices: Mapping<TokenId, Balance>,
        /// tokens which published for sale
        tokens_for_sale: Vec<TokenId>,
    }

    #[derive(
        scale::Decode,
        scale::Encode,
        Debug,
        PartialEq,
        ink_storage::traits::SpreadLayout,
        ink_storage::traits::PackedLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct ForSale {
        id: TokenId,
        price: Balance,
    }

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        LOL,
        CannotParseMetadata,
        NotOwner,
        NotApproved,
        TokenExists,
        TokenNotFound,
        CannotInsert,
        CannotFetchValue,
        NotAllowed,
        AlreadyForSale,
        NotForSale,
        NotEnoughSent,
        CannotMakeTransfer,
        CannotTransferToken,
    }

    #[derive(
        scale::Decode,
        scale::Encode,
        Debug,
        PartialEq,
        ink_storage::traits::SpreadLayout,
        ink_storage::traits::PackedLayout,
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub struct NftData {
        poebat: Option<ink_prelude::string::String>,
    }

    /// Event emitted when a token transfer occurs.
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: TokenId,
    }

    impl Erc721 {
        /// Creates a new ERC-721 token contract.
        #[ink(constructor)]
        pub fn new() -> Self {
            // This call is required to correctly initialize the
            // Mapping of the contract.
            ink_lang::utils::initialize_contract(|_: &mut Self| {
                // let caller = Self::env().caller();
            })
        }

        // #[ink(constructor)]
        // pub fn default() -> Self {
        //     ink_lang::utils::initialize_contract(|_| {})
        // }

        /// Returns the balance of the owner.
        ///
        /// This represents the amount of unique tokens the owner has.
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u32 {
            self.balance_of_or_zero(&owner)
        }

        /// Returns the owner of the token.
        #[ink(message)]
        pub fn owner_of(&self, id: TokenId) -> Option<AccountId> {
            self.token_owner.get(id)
        }

        /// Return all tokens of owner
        #[ink(message)]
        pub fn tokens_of_owner(&self, owner: AccountId) -> Vec<TokenId> {
            self.owned_tokens.get(owner).unwrap_or_default()
        }

        /// Return all tokens
        #[ink(message)]
        pub fn get_all_tokens(&self) -> Vec<(TokenId, NftData)> {
            self.all_tokens.iter().map(|id: &TokenId| (*id, self.token_data.get(id).unwrap())).collect()
        }

        /// Transfers the token from the caller to the given destination.
        #[ink(message)]
        pub fn transfer(&mut self, destination: AccountId, id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            self.transfer_token_from(&caller, &destination, id)?;
            Ok(())
        }

        /// Transfer owned token.
        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: TokenId,
        ) -> Result<(), Error> {
            self.transfer_token_from(&from, &to, id)?;
            Ok(())
        }

        /// Creates a new token.
        #[ink(message)]
        pub fn mint(&mut self, id: TokenId, data: NftData) -> Result<(), Error> {
            let caller = self.env().caller();

            self.add_token_to(&caller, id)?;
            self.token_data.insert(id, &data);
            self.all_tokens.push(id);
            self.env().emit_event(Transfer {
                from: Some(AccountId::from([0x0; 32])),
                to: Some(caller),
                id,
            });
            Ok(())
        }
        
        /// add token id for sale 
        #[ink(message)]
        pub fn publish_for_sale(&mut self, id: TokenId, price: Balance) -> Result<(), Error> {
            let caller = self.env().caller();
            if !self.exists(id) {
                return Err(Error::TokenNotFound);
            };
            if !self.is_owner_of(Some(caller), id) {
                return Err(Error::NotApproved);
            };
            if self.prices.contains(id) {
                return Err(Error::AlreadyForSale);
            }
            
            self.tokens_for_sale.push(id);
            self.prices.insert(id, &price);
            
            Ok(())
        }

        /// get all tokens which published for sale
        #[ink(message)]
        pub fn get_tokens_for_sale(&self) -> Vec<ForSale> {
            let mut res = Vec::new();
            for val in self.tokens_for_sale.iter() {
                if self.prices.contains(val) {
                    res.push(ForSale{
                        id: *val,
                        price: self.prices.get(val).unwrap(),
                    });
                }
            }
            res
        }

        /// remove tokens from saling
        #[ink(message)]
        pub fn remove_from_sale(&mut self, id: TokenId) -> Result<(), Error>{
            let caller = self.env().caller();
            if !self.exists(id) {
                return Err(Error::TokenNotFound);
            };
            if !self.is_owner_of(Some(caller), id) {
                return Err(Error::NotApproved);
            };
            if !self.prices.contains(id) {
                return Err(Error::NotForSale);
            }
            let index = self.tokens_for_sale.iter().position(|token| *token == id).ok_or(Error::CannotFetchValue)?;
            self.tokens_for_sale.remove(index);
            self.prices.remove(id);
            
            Ok(())
        }

        /// buy token for sale
        #[ink(message, payable)]
        pub fn buy_nft(&mut self, id: TokenId) -> Result<(), Error>{
            let caller = self.env().caller();
            if !self.exists(id) {
                return Err(Error::TokenNotFound);
            };
            if self.is_owner_of(Some(caller), id) { // ???? ?????????????? ?????? ???? ???? ??????????????????
                return Err(Error::NotApproved);
            };
            if !self.prices.contains(id) {
                return Err(Error::NotForSale);
            }
            let transfered_price = self.env().transferred_value();
            let token_price = self.prices.get(id).unwrap();
            if token_price > transfered_price {
                return Err(Error::NotEnoughSent);
            }

            let token_owner = self.owner_of(id).unwrap_or_default(); // ???? ???????????? ???? ?????????? ???? ??????????, ???? ?? ???????? ?????????? ???? ???????????? ???? ???????? ?????????????????? ????-????
            let err = self.env().transfer(token_owner, token_price);
            if err.is_err() {
                return Err(Error::CannotMakeTransfer);
            }
            
            self.transfer_token_from(&token_owner, &caller, id)?;

            let index = self.tokens_for_sale.iter().position(|token| *token == id).ok_or(Error::CannotFetchValue)?;
            self.tokens_for_sale.remove(index);
            self.prices.remove(id);
            
            Ok(())
        }

        /// Transfer owned token.
        #[ink(message)]
        pub fn get_nft_info(&self, id: TokenId) -> Result<NftData, Error> {
            self.token_data.get(id).ok_or(Error::TokenNotFound)
        }

        /// Deletes an existing token. Only the owner can burn the token.
        #[ink(message)]
        pub fn burn(&mut self, id: TokenId) -> Result<(), Error> {
            let caller = self.env().caller();
            let Self {
                token_owner,
                owned_tokens,
                owned_tokens_count,
                all_tokens,
                ..
            } = self;

            let owner = token_owner.get(id).ok_or(Error::TokenNotFound)?;
            if owner != caller {
                return Err(Error::NotOwner);
            };

            let count = owned_tokens_count
                .get(caller)
                .map(|c| c - 1)
                .ok_or(Error::CannotFetchValue)?;
            owned_tokens_count.insert(caller, &count);
            
            let mut tokens = owned_tokens
                .get(caller)
                .ok_or(Error::CannotFetchValue)?;
            
            let index = tokens.iter().position(|token| *token == id).ok_or(Error::CannotFetchValue)?;
            tokens.remove(index);
            owned_tokens.insert(caller, &tokens);

            let index = all_tokens.iter().position(|token| *token == id).ok_or(Error::CannotFetchValue)?;
            all_tokens.remove(index);

            token_owner.remove(id);

            self.env().emit_event(Transfer {
                from: Some(caller),
                to: Some(AccountId::from([0x0; 32])),
                id,
            });

            Ok(())
        }

        /// Transfers token `id` `from` the sender to the `to` `AccountId`.
        fn transfer_token_from(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            id: TokenId,
        ) -> Result<(), Error> {
            if !self.exists(id) {
                return Err(Error::TokenNotFound);
            };
            if !self.is_owner_of(Some(*from), id) {
                return Err(Error::NotApproved);
            };
            self.remove_token_from(from, id)?;
            self.add_token_to(to, id)?;
            self.env().emit_event(Transfer {
                from: Some(*from),
                to: Some(*to),
                id,
            });
            Ok(())
        }

        /// Removes token `id` from the owner.
        fn remove_token_from(&mut self, from: &AccountId, id: TokenId) -> Result<(), Error> {
            if !self.exists(id) {
                return Err(Error::TokenNotFound);
            }

            let Self {
                token_owner,
                owned_tokens,
                owned_tokens_count,
                ..
            } = self;

            let count = owned_tokens_count
                .get(from)
                .map(|c| c - 1)
                .ok_or(Error::CannotFetchValue)?;
            owned_tokens_count.insert(from, &count);

            let mut tokens = owned_tokens
                .get(from)
                .ok_or(Error::CannotFetchValue)?;
            
            let index = tokens.iter().position(|token| *token == id).ok_or(Error::CannotFetchValue)?;
            tokens.remove(index);
            owned_tokens.insert(from, &tokens);

            token_owner.remove(id);

            Ok(())
        }

        /// Adds the token `id` to the `to` AccountID.
        fn add_token_to(&mut self, to: &AccountId, id: TokenId) -> Result<(), Error> {
            let Self {
                token_owner,
                owned_tokens,
                owned_tokens_count,
                ..
            } = self;

            if token_owner.contains(id) {
                return Err(Error::TokenExists);
            }

            if *to == AccountId::from([0x0; 32]) {
                return Err(Error::NotAllowed);
            };

            let count = owned_tokens_count.get(to).map(|c| c + 1).unwrap_or(1);
            owned_tokens_count.insert(to, &count);

            let mut tokens = owned_tokens
                .get(to)
                .unwrap_or_default();
            tokens.push(id);
            owned_tokens.insert(to, &tokens);

            token_owner.insert(id, to);

            Ok(())
        }

        // Returns the total number of tokens from an account.
        fn balance_of_or_zero(&self, of: &AccountId) -> u32 {
            self.owned_tokens_count.get(of).unwrap_or(0)
        }

        /// Returns true if the `AccountId` `from` is the owner of token `id`
        fn is_owner_of(&self, from: Option<AccountId>, id: TokenId) -> bool {
            let owner = self.owner_of(id);
            from != Some(AccountId::from([0x0; 32])) && (from == owner)
        }

        /// Returns true if token `id` exists or false if it does not.
        fn exists(&self, id: TokenId) -> bool {
            self.token_owner.contains(id)
        }
    }

    /// Unit tests
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        #[ink_lang::test]
        fn mint_works() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = Erc721::new();
            // Token 1 does not exists.
            assert_eq!(erc721.owner_of(1), None);
            // Alice does not owns tokens.
            assert_eq!(erc721.balance_of(accounts.alice), 0);
            // Create token Id 1.
            assert_eq!(erc721.mint(1, NftData { poebat: None }), Ok(()));
            // Alice owns 1 token.
            assert_eq!(erc721.balance_of(accounts.alice), 1);
        }
        
        #[ink_lang::test]
        fn publish_for_sale_works() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = Erc721::new();

            assert_eq!(erc721.mint(1, NftData { poebat: None }), Ok(()));
            assert_eq!(erc721.mint(2, NftData { poebat: None }), Ok(()));
            assert_eq!(erc721.mint(3, NftData { poebat: None }), Ok(()));

            assert_eq!(erc721.publish_for_sale(1, 10), Ok(()));
            assert_eq!(erc721.get_tokens_for_sale(), vec![ForSale{id: 1, price: 10}]);

            assert_eq!(erc721.publish_for_sale(2, 100), Ok(()));
            assert_eq!(erc721.get_tokens_for_sale(), vec![ForSale{id: 1, price: 10}, ForSale{id: 2, price: 100}]);

            assert_eq!(erc721.remove_from_sale(1), Ok(()));
            assert_eq!(erc721.get_tokens_for_sale(), vec![ForSale{id: 2, price: 100}]);
        }

        #[ink_lang::test]
        fn buy_nft_works() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            let mut erc721 = Erc721::new();

            assert_eq!(erc721.mint(1, NftData { poebat: None }), Ok(()));

            assert_eq!(erc721.publish_for_sale(1, 10), Ok(()));
            assert_eq!(erc721.get_tokens_for_sale(), vec![ForSale{id: 1, price: 10}]);

            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);
            assert_eq!(erc721.is_owner_of(Some(accounts.alice), 1), true);
            ink_env::test::set_account_balance::<ink_env::DefaultEnvironment>(accounts.bob, 10);
            ink_env::test::set_value_transferred::<ink_env::DefaultEnvironment>(10);
            assert_eq!(erc721.buy_nft(1), Ok(()));
            
            assert_eq!(erc721.get_tokens_for_sale(), vec![]);
            assert_eq!(erc721.is_owner_of(Some(accounts.bob), 1), true);

        }

        #[ink_lang::test]
        fn get_all_tokens_works() {
            let accounts =
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = Erc721::new();
            // no token exists
            assert_eq!(erc721.get_all_tokens(), vec![]);
            // Create tokens
            assert_eq!(erc721.mint(1, NftData{poebat: Some("1".to_string())}), Ok(()));
            assert_eq!(erc721.mint(2, NftData{poebat: Some("2".to_string())}), Ok(()));

            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.bob);
            assert_eq!(erc721.mint(3, NftData{poebat: Some("3".to_string())}), Ok(()));

            // exists 3 tokens
            assert_eq!(erc721.get_all_tokens(), vec![(1, NftData{poebat: Some("1".to_string())}), (2, NftData{poebat: Some("2".to_string())}), (3, NftData{poebat: Some("3".to_string())})]);
            // burn token
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(accounts.alice);
            assert_eq!(erc721.burn(2), Ok(()));
            // exists 2 tokens
            assert_eq!(erc721.get_all_tokens(), vec![(1, NftData{poebat: Some("1".to_string())}), (3, NftData{poebat: Some("3".to_string())})]);
        }

        #[ink_lang::test]
        fn tokens_of_owner_works() {
            let accounts =
                ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = Erc721::new();
            // Token 1 does not exists.
            assert_eq!(erc721.owner_of(1), None);
            // Alice does not owns tokens.
            assert_eq!(erc721.tokens_of_owner(accounts.alice).len(), 0);
            // Create tokens
            assert_eq!(erc721.mint(1, NftData { poebat: None }), Ok(()));
            assert_eq!(erc721.mint(2, NftData { poebat: None }), Ok(()));
            assert_eq!(erc721.mint(3, NftData { poebat: None }), Ok(()));
            // Alice owns 1 token.
            assert_eq!(erc721.tokens_of_owner(accounts.alice), vec![1, 2, 3]);
        }

        #[ink_lang::test]
        fn mint_existing_should_fail() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = Erc721::new();
            // Create token Id 1.
            assert_eq!(erc721.mint(1, NftData { poebat: None }), Ok(()));
            // The first Transfer event takes place
            assert_eq!(1, ink_env::test::recorded_events().count());
            // Alice owns 1 token.
            assert_eq!(erc721.balance_of(accounts.alice), 1);
            // Alice owns token Id 1.
            assert_eq!(erc721.owner_of(1), Some(accounts.alice));
            // Cannot create  token Id if it exists.
            // Bob cannot own token Id 1.
            assert_eq!(erc721.mint(1, NftData { poebat: None }), Err(Error::TokenExists));
        }

        #[ink_lang::test]
        fn transfer_works() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = Erc721::new();
            // Create token Id 1 for Alice
            assert_eq!(erc721.mint(1, NftData { poebat: None }), Ok(()));
            // Alice owns token 1
            assert_eq!(erc721.balance_of(accounts.alice), 1);
            assert_eq!(erc721.owner_of(1), Some(accounts.alice));
            // Bob does not owns any token
            assert_eq!(erc721.balance_of(accounts.bob), 0);
            // The first Transfer event takes place
            assert_eq!(1, ink_env::test::recorded_events().count());
            // Alice transfers token 1 to Bob
            assert_eq!(erc721.transfer(accounts.bob, 1), Ok(()));
            // The second Transfer event takes place
            assert_eq!(2, ink_env::test::recorded_events().count());
            // Bob owns token 1
            assert_eq!(erc721.balance_of(accounts.bob), 1);
        }

        #[ink_lang::test]
        fn invalid_transfer_should_fail() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = Erc721::new();
            // Transfer token fails if it does not exists.
            assert_eq!(erc721.transfer(accounts.bob, 2), Err(Error::TokenNotFound));
            // Token Id 2 does not exists.
            assert_eq!(erc721.owner_of(2), None);
            // Create token Id 2.
            assert_eq!(erc721.mint(2, NftData { poebat: None }), Ok(()));
            // Alice owns 1 token.
            assert_eq!(erc721.balance_of(accounts.alice), 1);
            // Token Id 2 is owned by Alice.
            assert_eq!(erc721.owner_of(2), Some(accounts.alice));
            // Set Bob as caller
            set_caller(accounts.bob);
            // Bob cannot transfer not owned tokens.
            assert_eq!(erc721.transfer(accounts.eve, 2), Err(Error::NotApproved));
        }

        #[ink_lang::test]
        fn token_metadate() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = Erc721::new();
            // Transfer token fails if it does not exists.
            assert_eq!(erc721.mint(2, NftData { poebat: Some("lol".to_string()) }), Ok(()));
            // Alice owns 1 token.
            assert_eq!(
                erc721.get_nft_info(2),
                Ok(NftData { poebat: Some("lol".to_string())})
            );

            assert_eq!(
                erc721.get_nft_info(3),
                Err(Error::TokenNotFound)
            );
        }

        #[ink_lang::test]
        fn burn_works() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = Erc721::new();
            // Create token Id 1 for Alice
            assert_eq!(erc721.mint(1, NftData { poebat: None }), Ok(()));
            // Alice owns 1 token.
            assert_eq!(erc721.balance_of(accounts.alice), 1);
            // Alice owns token Id 1.
            assert_eq!(erc721.owner_of(1), Some(accounts.alice));
            // Destroy token Id 1.
            assert_eq!(erc721.burn(1), Ok(()));
            // Alice does not owns tokens.
            assert_eq!(erc721.balance_of(accounts.alice), 0);
            // Token Id 1 does not exists
            assert_eq!(erc721.owner_of(1), None);
        }

        #[ink_lang::test]
        fn burn_fails_token_not_found() {
            // Create a new contract instance.
            let mut erc721 = Erc721::new();
            // Try burning a non existent token
            assert_eq!(erc721.burn(1), Err(Error::TokenNotFound));
        }

        #[ink_lang::test]
        fn burn_fails_not_owner() {
            let accounts = ink_env::test::default_accounts::<ink_env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut erc721 = Erc721::new();
            // Create token Id 1 for Alice
            assert_eq!(erc721.mint(1, NftData { poebat: None }), Ok(()));
            // Try burning this token with a different account
            set_caller(accounts.eve);
            assert_eq!(erc721.burn(1), Err(Error::NotOwner));
        }

        fn set_caller(sender: AccountId) {
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(sender);
        }
    }
}
