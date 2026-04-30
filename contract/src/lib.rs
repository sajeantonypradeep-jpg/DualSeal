#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String, Vec,
};

/// A dual-signed commitment between two Stellar addresses.
#[contracttype]
#[derive(Clone)]
pub struct Vow {
    pub id: u64,
    pub proposer: Address,
    pub partner: Address,
    pub vow_text: String,
    pub proposer_signed: bool,
    pub partner_signed: bool,
    pub sealed: bool,
    pub timestamp: u64,
}

/// Storage keys used by the contract.
#[contracttype]
pub enum DataKey {
    Vow(u64),
    VowCount,
    WalletVows(Address),
}

/// Maximum number of vows allowed.
const MAX_VOWS: u64 = 10_000;

#[contract]
pub struct DualSealContract;

#[contractimpl]
impl DualSealContract {
    /// Create a new vow proposal. The proposer signs immediately;
    /// the partner must later call `seal_vow` to finalize.
    ///
    /// Returns the newly created vow ID.
    pub fn propose_vow(
        env: Env,
        proposer: Address,
        partner: Address,
        vow_text: String,
    ) -> u64 {
        proposer.require_auth();

        // Increment vow counter
        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::VowCount)
            .unwrap_or(0u64);

        let vow_id = count + 1;
        assert!(vow_id <= MAX_VOWS, "Maximum vow count reached");

        let vow = Vow {
            id: vow_id,
            proposer: proposer.clone(),
            partner: partner.clone(),
            vow_text,
            proposer_signed: true,
            partner_signed: false,
            sealed: false,
            timestamp: env.ledger().timestamp(),
        };

        // Store vow
        env.storage()
            .persistent()
            .set(&DataKey::Vow(vow_id), &vow);

        // Track vow IDs per wallet (proposer)
        let mut proposer_vows: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::WalletVows(proposer.clone()))
            .unwrap_or(Vec::new(&env));
        proposer_vows.push_back(vow_id);
        env.storage()
            .persistent()
            .set(&DataKey::WalletVows(proposer), &proposer_vows);

        // Track vow IDs per wallet (partner)
        let mut partner_vows: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::WalletVows(partner.clone()))
            .unwrap_or(Vec::new(&env));
        partner_vows.push_back(vow_id);
        env.storage()
            .persistent()
            .set(&DataKey::WalletVows(partner), &partner_vows);

        // Update global vow count
        env.storage()
            .instance()
            .set(&DataKey::VowCount, &vow_id);

        // Emit event
        env.events().publish(
            (symbol_short!("vow_prop"),),
            (vow_id,),
        );

        vow_id
    }

    /// Seal an existing vow. Only the designated partner can seal,
    /// and only if the vow has not already been sealed.
    pub fn seal_vow(env: Env, vow_id: u64, signer: Address) {
        signer.require_auth();

        let mut vow: Vow = env
            .storage()
            .persistent()
            .get(&DataKey::Vow(vow_id))
            .expect("Vow not found");

        assert!(!vow.sealed, "Vow already sealed");
        assert!(
            signer == vow.partner,
            "Only the partner can seal this vow"
        );
        assert!(!vow.partner_signed, "Partner already signed");

        vow.partner_signed = true;
        vow.sealed = true;

        env.storage()
            .persistent()
            .set(&DataKey::Vow(vow_id), &vow);

        // Emit event
        env.events().publish(
            (symbol_short!("vow_seal"),),
            (vow_id,),
        );
    }

    /// Read a vow by its ID. Panics if the vow does not exist.
    pub fn get_vow(env: Env, vow_id: u64) -> Vow {
        env.storage()
            .persistent()
            .get(&DataKey::Vow(vow_id))
            .expect("Vow not found")
    }

    /// Return all vow IDs associated with a wallet address.
    pub fn get_wallet_vows(env: Env, wallet: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::WalletVows(wallet))
            .unwrap_or(Vec::new(&env))
    }

    /// Return the total number of vows ever created.
    pub fn vow_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::VowCount)
            .unwrap_or(0u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events};

    fn setup() -> (Env, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, DualSealContract);
        (env, contract_id)
    }

    #[test]
    fn test_propose_vow_returns_id() {
        let (env, contract_id) = setup();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        let vow_id: u64 = env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "We commit to building together"),
            )
        });

        assert_eq!(vow_id, 1);
    }

    #[test]
    fn test_propose_vow_stores_correctly() {
        let (env, contract_id) = setup();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "We commit to building together"),
            );
        });

        let vow: Vow = env.as_contract(&contract_id, || {
            DualSealContract::get_vow(env.clone(), 1)
        });

        assert_eq!(vow.id, 1);
        assert_eq!(vow.proposer_signed, true);
        assert_eq!(vow.partner_signed, false);
        assert_eq!(vow.sealed, false);
    }

    #[test]
    fn test_propose_vow_increments_count() {
        let (env, contract_id) = setup();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        for i in 1..=5 {
            env.as_contract(&contract_id, || {
                DualSealContract::propose_vow(
                    env.clone(),
                    proposer.clone(),
                    partner.clone(),
                    String::from_str(&env, "Test vow"),
                );
            });

            let count: u64 = env.as_contract(&contract_id, || {
                DualSealContract::vow_count(env.clone())
            });
            assert_eq!(count, i);
        }
    }

    #[test]
    fn test_propose_vow_tracks_wallet_vows() {
        let (env, contract_id) = setup();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "First vow"),
            );
        });

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Second vow"),
            );
        });

        let proposer_vows: Vec<u64> = env.as_contract(&contract_id, || {
            DualSealContract::get_wallet_vows(env.clone(), proposer.clone())
        });
        assert_eq!(proposer_vows.len(), 2);

        let partner_vows: Vec<u64> = env.as_contract(&contract_id, || {
            DualSealContract::get_wallet_vows(env.clone(), partner.clone())
        });
        assert_eq!(partner_vows.len(), 2);
    }

    #[test]
    fn test_propose_vow_emits_event() {
        let (env, contract_id) = setup();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Test vow"),
            );
        });

        let events = env.events().all();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_seal_vow_marks_sealed() {
        let (env, contract_id) = setup();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Test vow"),
            );

            DualSealContract::seal_vow(env.clone(), 1, partner.clone());
        });

        let vow: Vow = env.as_contract(&contract_id, || {
            DualSealContract::get_vow(env.clone(), 1)
        });

        assert_eq!(vow.sealed, true);
        assert_eq!(vow.partner_signed, true);
    }

    #[test]
    fn test_seal_vow_emits_event() {
        let (env, contract_id) = setup();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Test vow"),
            );
            DualSealContract::seal_vow(env.clone(), 1, partner.clone());
        });

        let events = env.events().all();
        assert_eq!(events.len(), 2); // propose + seal
    }

    #[test]
    fn test_get_vow_returns_correct_data() {
        let (env, contract_id) = setup();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "We build together"),
            );
        });

        let vow: Vow = env.as_contract(&contract_id, || {
            DualSealContract::get_vow(env.clone(), 1)
        });

        assert_eq!(vow.id, 1);
    }

    #[test]
    fn test_get_wallet_vows_empty_for_new_wallet() {
        let (env, contract_id) = setup();
        let wallet = Address::generate(&env);

        let vows: Vec<u64> = env.as_contract(&contract_id, || {
            DualSealContract::get_wallet_vows(env.clone(), wallet.clone())
        });

        assert_eq!(vows.len(), 0);
    }

    #[test]
    fn test_vow_count_zero_initially() {
        let (env, contract_id) = setup();

        let count: u64 = env.as_contract(&contract_id, || {
            DualSealContract::vow_count(env.clone())
        });

        assert_eq!(count, 0);
    }

    #[test]
    fn test_full_propose_and_seal_workflow() {
        let (env, contract_id) = setup();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        // Propose
        env.as_contract(&contract_id, || {
            let vow_id = DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Our eternal commitment"),
            );
            assert_eq!(vow_id, 1);
        });

        // Verify initial state
        let vow_before: Vow = env.as_contract(&contract_id, || {
            DualSealContract::get_vow(env.clone(), 1)
        });
        assert_eq!(vow_before.proposer_signed, true);
        assert_eq!(vow_before.partner_signed, false);
        assert_eq!(vow_before.sealed, false);

        // Seal
        env.as_contract(&contract_id, || {
            DualSealContract::seal_vow(env.clone(), 1, partner.clone());
        });

        // Verify final state
        let vow_after: Vow = env.as_contract(&contract_id, || {
            DualSealContract::get_vow(env.clone(), 1)
        });
        assert_eq!(vow_after.proposer_signed, true);
        assert_eq!(vow_after.partner_signed, true);
        assert_eq!(vow_after.sealed, true);

        // Verify count
        let count: u64 = env.as_contract(&contract_id, || {
            DualSealContract::vow_count(env.clone())
        });
        assert_eq!(count, 1);
    }
}
