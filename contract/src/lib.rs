#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String, Vec, Map,
};

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

#[contracttype]
pub enum DataKey {
    Vow(u64),
    VowCount,
    WalletVows(Address),
}

const MAX_VOWS: u64 = 10_000;

#[contract]
pub struct ChainVowContract;

#[contractimpl]
impl ChainVowContract {
    /// Propose a vow between two addresses
    pub fn propose_vow(
        env: Env,
        proposer: Address,
        partner: Address,
        vow_text: String,
    ) -> u64 {
        proposer.require_auth();

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::VowCount)
            .unwrap_or(0u64);

        let vow_id = count + 1;

        let vow = Vow {
            id: vow_id,
            proposer: proposer.clone(),
            partner: partner.clone(),
            vow_text,
            proposer_signed: true, // proposer signs on creation
            partner_signed: false,
            sealed: false,
            timestamp: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Vow(vow_id), &vow);

        // Track vow IDs per wallet
        let mut proposer_vows: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::WalletVows(proposer.clone()))
            .unwrap_or(Vec::new(&env));
        proposer_vows.push_back(vow_id);
        env.storage()
            .persistent()
            .set(&DataKey::WalletVows(proposer), &proposer_vows);

        let mut partner_vows: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::WalletVows(partner.clone()))
            .unwrap_or(Vec::new(&env));
        partner_vows.push_back(vow_id);
        env.storage()
            .persistent()
            .set(&DataKey::WalletVows(partner), &partner_vows);

        env.storage()
            .instance()
            .set(&DataKey::VowCount, &vow_id);

        env.events().publish(
            (symbol_short!("vow_prop"),),
            (vow_id,),
        );

        vow_id
    }

    /// Partner signs/accepts the vow — seals it forever
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

        env.events().publish(
            (symbol_short!("vow_seal"),),
            (vow_id,),
        );
    }

    /// Read a vow by ID
    pub fn get_vow(env: Env, vow_id: u64) -> Vow {
        env.storage()
            .persistent()
            .get(&DataKey::Vow(vow_id))
            .expect("Vow not found")
    }

    /// Get all vow IDs for a wallet
    pub fn get_wallet_vows(env: Env, wallet: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::WalletVows(wallet))
            .unwrap_or(Vec::new(&env))
    }

    /// Total vows ever created
    pub fn vow_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::VowCount)
            .unwrap_or(0u64)
    }
}
