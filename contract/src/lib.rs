#![no_std]
use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype,
    symbol_short, Address, Env, String, Vec,
};

/// Client trait for cross-contract calls from DualSeal to Registry.
#[contractclient(name = "RegistryClient")]
pub trait RegistryClientTrait {
    fn register_vow(
        env: Env,
        vow_id: u64,
        proposer: Address,
        partner: Address,
        stake_amount: i128,
    );
    fn record_seal(env: Env, vow_id: u64, sealer: Address, partner_stake: i128);
}

// ── Data Types ─────────────────────────────────────────────────────────────

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
    pub stake_amount: i128,
    pub partner_stake: i128,
    pub timestamp: u64,
}

/// A Registry entry mirroring a vow for cross-contract tracking.
#[contracttype]
#[derive(Clone)]
pub struct RegistryEntry {
    pub vow_id: u64,
    pub proposer: Address,
    pub partner: Address,
    pub total_stake: i128,
    pub registered: bool,
    pub sealed: bool,
    pub sealed_timestamp: u64,
}

/// Storage keys for the DualSeal contract.
#[contracttype]
pub enum DataKey {
    Vow(u64),
    VowCount,
    WalletVows(Address),
}

/// Storage keys for the Registry contract.
#[contracttype]
pub enum RegistryKey {
    Entry(u64),
    RegistryCount,
}

/// Error codes for the DualSeal contract.
#[contracterror]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum DualSealError {
    VowNotFound = 1,
    VowAlreadySealed = 2,
    NotPartner = 3,
    PartnerAlreadySigned = 4,
    MaxVowsReached = 5,
    StakeTooLow = 6,
}

/// Error codes for the Registry contract.
#[contracterror]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RegistryError {
    EntryNotFound = 1,
    AlreadySealed = 2,
    NotPartner = 3,
}

/// Maximum number of vows allowed.
const MAX_VOWS: u64 = 10_000;

/// Minimum stake required to propose a vow (in stroops).
const MIN_STAKE: i128 = 1_000_000; // 0.1 XLM

// ── DualSeal Contract ──────────────────────────────────────────────────────

#[contract]
pub struct DualSealContract;

#[contractimpl]
impl DualSealContract {
    /// Create a new vow proposal with an optional stake deposit.
    /// The proposer signs immediately; the partner must later call
    /// `seal_vow` to finalize.
    ///
    /// Returns the newly created vow ID.
    pub fn propose_vow(
        env: Env,
        proposer: Address,
        partner: Address,
        vow_text: String,
        stake_amount: i128,
    ) -> u64 {
        proposer.require_auth();

        assert!(stake_amount >= MIN_STAKE, "Stake too low");

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
            stake_amount,
            partner_stake: 0,
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
            .set(&DataKey::WalletVows(proposer.clone()), &proposer_vows);

        // Track vow IDs per wallet (partner)
        let mut partner_vows: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::WalletVows(partner.clone()))
            .unwrap_or(Vec::new(&env));
        partner_vows.push_back(vow_id);
        env.storage()
            .persistent()
            .set(&DataKey::WalletVows(partner.clone()), &partner_vows);

        // Update global vow count
        env.storage()
            .instance()
            .set(&DataKey::VowCount, &vow_id);

        // Inter-contract call: register the vow in the Registry
        let registry_id = env.storage().instance().get(&symbol_short!("reg_id")).expect("Registry not configured");
        let client = RegistryClient::new(&env, &registry_id);
        client.register_vow(&vow_id, &proposer, &partner, &stake_amount);

        // Emit event
        env.events().publish(
            (symbol_short!("vow_prop"),),
            (vow_id, stake_amount),
        );

        vow_id
    }

    /// Seal an existing vow with a matching stake from the partner.
    /// Only the designated partner can seal, and only if the vow has
    /// not already been sealed.
    pub fn seal_vow(
        env: Env,
        vow_id: u64,
        signer: Address,
        partner_stake: i128,
    ) {
        signer.require_auth();

        let mut vow: Vow = env
            .storage()
            .persistent()
            .get(&DataKey::Vow(vow_id))
            .expect("Vow not found");

        assert!(!vow.sealed, "Vow already sealed");
        assert!(signer == vow.partner, "Only the partner can seal this vow");
        assert!(!vow.partner_signed, "Partner already signed");
        assert!(partner_stake >= MIN_STAKE, "Stake too low");

        vow.partner_signed = true;
        vow.sealed = true;
        vow.partner_stake = partner_stake;

        env.storage()
            .persistent()
            .set(&DataKey::Vow(vow_id), &vow);

        // Inter-contract call: record the seal in the Registry
        let registry_id = env.storage().instance().get(&symbol_short!("reg_id")).expect("Registry not configured");
        let client = RegistryClient::new(&env, &registry_id);
        client.record_seal(&vow_id, &signer, &partner_stake);

        // Emit event
        env.events().publish(
            (symbol_short!("vow_seal"),),
            (vow_id, partner_stake),
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

    /// Set the Registry contract address. Must be called once during setup.
    pub fn set_registry(env: Env, admin: Address, registry_id: Address) {
        admin.require_auth();
        env.storage()
            .instance()
            .set(&symbol_short!("reg_id"), &registry_id);
    }

    /// Get the configured Registry contract address.
    pub fn get_registry(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&symbol_short!("reg_id"))
            .expect("Registry not configured")
    }
}

// ── Registry Contract ──────────────────────────────────────────────────────

/// A secondary contract that tracks all vows for cross-contract
/// verification and auditing. DualSeal calls this contract whenever
/// a vow is proposed or sealed.
#[contract]
pub struct RegistryContract;

#[contractimpl]
impl RegistryContract {
    /// Register a new vow entry. Called by DualSeal when a vow is proposed.
    pub fn register_vow(
        env: Env,
        vow_id: u64,
        proposer: Address,
        partner: Address,
        stake_amount: i128,
    ) {
        let entry = RegistryEntry {
            vow_id,
            proposer,
            partner,
            total_stake: stake_amount,
            registered: true,
            sealed: false,
            sealed_timestamp: 0,
        };

        env.storage()
            .persistent()
            .set(&RegistryKey::Entry(vow_id), &entry);

        // Update registry count
        let count: u64 = env
            .storage()
            .instance()
            .get(&RegistryKey::RegistryCount)
            .unwrap_or(0u64);
        env.storage()
            .instance()
            .set(&RegistryKey::RegistryCount, &(count + 1));

        env.events().publish(
            (symbol_short!("reg_new"),),
            (vow_id,),
        );
    }

    /// Record a vow seal. Called by DualSeal when a partner seals.
    pub fn record_seal(
        env: Env,
        vow_id: u64,
        sealer: Address,
        partner_stake: i128,
    ) {
        let mut entry: RegistryEntry = env
            .storage()
            .persistent()
            .get(&RegistryKey::Entry(vow_id))
            .expect("Registry entry not found");

        assert!(!entry.sealed, "Registry entry already sealed");
        assert!(sealer == entry.partner, "Only the partner can record seal");

        entry.sealed = true;
        entry.total_stake += partner_stake;
        entry.sealed_timestamp = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&RegistryKey::Entry(vow_id), &entry);

        env.events().publish(
            (symbol_short!("reg_seal"),),
            (vow_id,),
        );
    }

    /// Check if a vow is registered in the Registry.
    pub fn is_registered(env: Env, vow_id: u64) -> bool {
        let entry: Option<RegistryEntry> = env
            .storage()
            .persistent()
            .get(&RegistryKey::Entry(vow_id));
        entry.is_some()
    }

    /// Get a Registry entry by vow ID.
    pub fn get_entry(env: Env, vow_id: u64) -> RegistryEntry {
        env.storage()
            .persistent()
            .get(&RegistryKey::Entry(vow_id))
            .expect("Registry entry not found")
    }

    /// Total vows registered in this contract.
    pub fn registry_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&RegistryKey::RegistryCount)
            .unwrap_or(0u64)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events, Ledger, LedgerInfo};

    fn setup_with_registry() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths_allowing_non_root_auth();

        // Set up ledger for timestamp
        env.ledger().set(LedgerInfo {
            timestamp: 1000,
            protocol_version: 22,
            sequence_number: 100,
            network_id: [0u8; 32],
            base_reserve: 10,
            min_persistent_entry_ttl: 100,
            min_temp_entry_ttl: 100,
            max_entry_ttl: 1000,
        });

        let registry_id = env.register_contract(None, RegistryContract);
        let dual_seal_id = env.register_contract(None, DualSealContract);

        let admin = Address::generate(&env);

        // Configure registry on DualSeal
        env.as_contract(&dual_seal_id, || {
            DualSealContract::set_registry(env.clone(), admin.clone(), registry_id.clone());
        });

        (env, dual_seal_id, registry_id)
    }

    const DEFAULT_STAKE: i128 = 5_000_000; // 0.5 XLM

    // ── propose_vow tests ─────────────────────────────────────────────────

    #[test]
    fn test_propose_vow_returns_id() {
        let (env, contract_id, _) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        let vow_id: u64 = env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "We commit to building together"),
                DEFAULT_STAKE,
            )
        });

        assert_eq!(vow_id, 1);
    }

    #[test]
    fn test_propose_vow_stores_correctly() {
        let (env, contract_id, _) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "We commit to building together"),
                DEFAULT_STAKE,
            );
        });

        let vow: Vow = env.as_contract(&contract_id, || {
            DualSealContract::get_vow(env.clone(), 1)
        });

        assert_eq!(vow.id, 1);
        assert_eq!(vow.proposer_signed, true);
        assert_eq!(vow.partner_signed, false);
        assert_eq!(vow.sealed, false);
        assert_eq!(vow.stake_amount, DEFAULT_STAKE);
        assert_eq!(vow.partner_stake, 0);
    }

    #[test]
    fn test_propose_vow_rejects_low_stake() {
        let (env, contract_id, _) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            env.as_contract(&contract_id, || {
                DualSealContract::propose_vow(
                    env.clone(),
                    proposer.clone(),
                    partner.clone(),
                    String::from_str(&env, "Test vow"),
                    500_000, // below MIN_STAKE
                );
            });
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_propose_vow_increments_count() {
        let (env, contract_id, _) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        for i in 1..=5 {
            env.as_contract(&contract_id, || {
                DualSealContract::propose_vow(
                    env.clone(),
                    proposer.clone(),
                    partner.clone(),
                    String::from_str(&env, "Test vow"),
                    DEFAULT_STAKE,
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
        let (env, contract_id, _) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "First vow"),
                DEFAULT_STAKE,
            );
        });

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Second vow"),
                DEFAULT_STAKE,
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

    // ── seal_vow tests ────────────────────────────────────────────────────

    #[test]
    fn test_seal_vow_marks_sealed() {
        let (env, contract_id, _) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Test vow"),
                DEFAULT_STAKE,
            );

            DualSealContract::seal_vow(env.clone(), 1, partner.clone(), DEFAULT_STAKE);
        });

        let vow: Vow = env.as_contract(&contract_id, || {
            DualSealContract::get_vow(env.clone(), 1)
        });

        assert_eq!(vow.sealed, true);
        assert_eq!(vow.partner_signed, true);
        assert_eq!(vow.partner_stake, DEFAULT_STAKE);
    }

    #[test]
    fn test_seal_vow_rejects_non_partner() {
        let (env, contract_id, _) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);
        let other = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Test vow"),
                DEFAULT_STAKE,
            );
        });

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            env.as_contract(&contract_id, || {
                DualSealContract::seal_vow(env.clone(), 1, other.clone(), DEFAULT_STAKE);
            });
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_seal_vow_revents_double_seal() {
        let (env, contract_id, _) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Test vow"),
                DEFAULT_STAKE,
            );
            DualSealContract::seal_vow(env.clone(), 1, partner.clone(), DEFAULT_STAKE);
        });

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            env.as_contract(&contract_id, || {
                DualSealContract::seal_vow(env.clone(), 1, partner.clone(), DEFAULT_STAKE);
            });
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_seal_vow_emits_event() {
        let (env, contract_id, _) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Test vow"),
                DEFAULT_STAKE,
            );
            DualSealContract::seal_vow(env.clone(), 1, partner.clone(), DEFAULT_STAKE);
        });

        let events = env.events().all();
        // propose + seal + reg_new + reg_seal = 4 events
        assert!(events.len() >= 4);
    }

    // ── Registry tests ────────────────────────────────────────────────────

    #[test]
    fn test_registry_records_proposal() {
        let (env, dual_seal_id, registry_id) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&dual_seal_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Test vow"),
                DEFAULT_STAKE,
            );
        });

        // Verify registry entry was created via inter-contract call
        let entry: RegistryEntry = env.as_contract(&registry_id, || {
            RegistryContract::get_entry(env.clone(), 1)
        });

        assert_eq!(entry.vow_id, 1);
        assert_eq!(entry.registered, true);
        assert_eq!(entry.sealed, false);
        assert_eq!(entry.total_stake, DEFAULT_STAKE);
    }

    #[test]
    fn test_registry_records_seal() {
        let (env, dual_seal_id, registry_id) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&dual_seal_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Test vow"),
                DEFAULT_STAKE,
            );
            DualSealContract::seal_vow(env.clone(), 1, partner.clone(), DEFAULT_STAKE);
        });

        let entry: RegistryEntry = env.as_contract(&registry_id, || {
            RegistryContract::get_entry(env.clone(), 1)
        });

        assert_eq!(entry.sealed, true);
        assert_eq!(entry.total_stake, DEFAULT_STAKE * 2); // proposer + partner stakes
        assert!(entry.sealed_timestamp > 0);
    }

    #[test]
    fn test_registry_is_registered() {
        let (env, dual_seal_id, registry_id) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        // Before proposal: not registered
        let registered_before: bool = env.as_contract(&registry_id, || {
            RegistryContract::is_registered(env.clone(), 1)
        });
        assert_eq!(registered_before, false);

        // After proposal: registered
        env.as_contract(&dual_seal_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Test vow"),
                DEFAULT_STAKE,
            );
        });

        let registered_after: bool = env.as_contract(&registry_id, || {
            RegistryContract::is_registered(env.clone(), 1)
        });
        assert_eq!(registered_after, true);
    }

    #[test]
    fn test_registry_count() {
        let (env, dual_seal_id, registry_id) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        for i in 1..=3 {
            env.as_contract(&dual_seal_id, || {
                DualSealContract::propose_vow(
                    env.clone(),
                    proposer.clone(),
                    partner.clone(),
                    String::from_str(&env, "Test vow"),
                    DEFAULT_STAKE,
                );
            });

            let count: u64 = env.as_contract(&registry_id, || {
                RegistryContract::registry_count(env.clone())
            });
            assert_eq!(count, i);
        }
    }

    // ── Inter-contract call tests ─────────────────────────────────────────

    #[test]
    fn test_inter_contract_call_on_propose() {
        let (env, dual_seal_id, registry_id) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        // Propose a vow - should trigger inter-contract call
        env.as_contract(&dual_seal_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Cross-contract test"),
                DEFAULT_STAKE,
            );
        });

        // Verify the Registry contract has the entry
        let is_reg: bool = env.as_contract(&registry_id, || {
            RegistryContract::is_registered(env.clone(), 1)
        });
        assert!(is_reg);
    }

    #[test]
    fn test_inter_contract_call_on_seal() {
        let (env, dual_seal_id, registry_id) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        // Propose and seal
        env.as_contract(&dual_seal_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Cross-contract seal test"),
                DEFAULT_STAKE,
            );
            DualSealContract::seal_vow(env.clone(), 1, partner.clone(), DEFAULT_STAKE);
        });

        // Verify Registry reflects the sealed state
        let entry: RegistryEntry = env.as_contract(&registry_id, || {
            RegistryContract::get_entry(env.clone(), 1)
        });
        assert!(entry.sealed);
        assert_eq!(entry.total_stake, DEFAULT_STAKE * 2);
    }

    // ── get_vow tests ─────────────────────────────────────────────────────

    #[test]
    fn test_get_vow_returns_correct_data() {
        let (env, contract_id, _) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        env.as_contract(&contract_id, || {
            DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "We build together"),
                DEFAULT_STAKE,
            );
        });

        let vow: Vow = env.as_contract(&contract_id, || {
            DualSealContract::get_vow(env.clone(), 1)
        });

        assert_eq!(vow.id, 1);
    }

    // ── get_wallet_vows tests ─────────────────────────────────────────────

    #[test]
    fn test_get_wallet_vows_empty_for_new_wallet() {
        let (env, contract_id, _) = setup_with_registry();
        let wallet = Address::generate(&env);

        let vows: Vec<u64> = env.as_contract(&contract_id, || {
            DualSealContract::get_wallet_vows(env.clone(), wallet.clone())
        });

        assert_eq!(vows.len(), 0);
    }

    // ── vow_count tests ───────────────────────────────────────────────────

    #[test]
    fn test_vow_count_zero_initially() {
        let (env, contract_id, _) = setup_with_registry();

        let count: u64 = env.as_contract(&contract_id, || {
            DualSealContract::vow_count(env.clone())
        });

        assert_eq!(count, 0);
    }

    // ── full workflow test ────────────────────────────────────────────────

    #[test]
    fn test_full_propose_and_seal_workflow() {
        let (env, dual_seal_id, registry_id) = setup_with_registry();
        let proposer = Address::generate(&env);
        let partner = Address::generate(&env);

        // Propose
        env.as_contract(&dual_seal_id, || {
            let vow_id = DualSealContract::propose_vow(
                env.clone(),
                proposer.clone(),
                partner.clone(),
                String::from_str(&env, "Our eternal commitment"),
                DEFAULT_STAKE,
            );
            assert_eq!(vow_id, 1);
        });

        // Verify initial state
        let vow_before: Vow = env.as_contract(&dual_seal_id, || {
            DualSealContract::get_vow(env.clone(), 1)
        });
        assert_eq!(vow_before.proposer_signed, true);
        assert_eq!(vow_before.partner_signed, false);
        assert_eq!(vow_before.sealed, false);
        assert_eq!(vow_before.stake_amount, DEFAULT_STAKE);

        // Seal
        env.as_contract(&dual_seal_id, || {
            DualSealContract::seal_vow(env.clone(), 1, partner.clone(), DEFAULT_STAKE);
        });

        // Verify final state
        let vow_after: Vow = env.as_contract(&dual_seal_id, || {
            DualSealContract::get_vow(env.clone(), 1)
        });
        assert_eq!(vow_after.proposer_signed, true);
        assert_eq!(vow_after.partner_signed, true);
        assert_eq!(vow_after.sealed, true);
        assert_eq!(vow_after.partner_stake, DEFAULT_STAKE);

        // Verify count
        let count: u64 = env.as_contract(&dual_seal_id, || {
            DualSealContract::vow_count(env.clone())
        });
        assert_eq!(count, 1);

        // Verify Registry reflects final state
        let reg_entry: RegistryEntry = env.as_contract(&registry_id, || {
            RegistryContract::get_entry(env.clone(), 1)
        });
        assert!(reg_entry.sealed);
        assert_eq!(reg_entry.total_stake, DEFAULT_STAKE * 2);
    }
}
