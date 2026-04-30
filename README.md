# DualSeal Protocol ✦

> **Two-phase signature smart contract on Soroban enabling immutable, dual-signed commitments between Stellar wallets.**

Two wallets. One vow. Immutable forever.

DualSeal Protocol lets any two Stellar wallet holders write a mutual commitment on-chain. The proposer writes the vow text and sets the partner's address. The partner seals it — a two-signature ceremony that locks the text to both addresses permanently in a Soroban smart contract.

---

## Live Links

| | |
|---|---|
| **Frontend** | `https://chainvow.vercel.app` |
| **Contract on Stellar Expert** | `https://stellar.expert/explorer/testnet/contract/CBIVYOVF66XZYUAF3YG6NKJI4R366HLUTHGL2C3WMBSO5HPVO5FNUBZU` |

---

## Tech Stack

| Layer | Tech |
|---|---|
| Smart Contract | Rust + Soroban SDK v22 |
| Network | Stellar Testnet |
| Frontend | React 18 + Vite |
| Wallet | Freighter via `@stellar/freighter-api` |
| RPC | Soroban RPC (`soroban-testnet.stellar.org`) |
| Hosting | Vercel |

---

## Why This Project Matters

This project turns a familiar real-world workflow into a verifiable on-chain primitive on Stellar: transparent state transitions, user-authenticated actions, and deterministic outcomes.

## Architecture

- **Smart Contract Layer**: Soroban contract enforces business rules, authorization, and state transitions. Registry contract provides cross-contract vow tracking and combined stake accounting.
- **Client Layer**: React + Vite frontend handles wallet UX, transaction composition, and real-time status views.
- **Wallet/Auth Layer**: Freighter signs every state-changing action so operations are attributable and non-repudiable.
- **Infra Layer**: Stellar Testnet + Soroban RPC for execution; Vercel for frontend hosting; GitHub Actions for CI/CD.

## CI/CD

This project uses GitHub Actions for continuous integration. On every push and pull request to `main`/`master`, the pipeline automatically:

- Builds the Soroban contract to WASM
- Runs all contract unit tests (19 tests)
- Installs frontend dependencies and builds the React app

Any failure in build, test, or frontend compilation will block the pipeline, ensuring that only production-ready code merges.

[![CI/CD](https://github.com/YOUR_USERNAME/dual-seal/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/dual-seal/actions/workflows/ci.yml)

---

## Run Locally

### Prerequisites
- [Rust](https://rustup.rs/) + `wasm32-unknown-unknown` target
- [Stellar CLI](https://github.com/stellar/stellar-cli) v25+
- [Node.js](https://nodejs.org/) 20+
- [Freighter wallet](https://freighter.app/) browser extension

### Deploy Contract + Run Frontend

```bash
# 1. Deploy contract to testnet (takes ~2 min)
chmod +x scripts/deploy.sh && ./scripts/deploy.sh

# 2. Install and run frontend
cd frontend
npm install
npm run dev
# → http://localhost:5173
```

### Publish to GitHub + Vercel

```bash
chmod +x scripts/publish.sh && ./scripts/publish.sh
```

---

## Architecture

```
User (Freighter Wallet)
        │
        ▼
React Frontend (Vite)
        │  @stellar/freighter-api  ← signs transactions
        │  @stellar/stellar-sdk   ← builds + submits txs
        ▼
Soroban RPC (testnet)
        │
        ▼
DualSeal Contract (WASM on Stellar testnet)
        │
        ├── propose_vow()  → stores Vow struct, marks proposer_signed=true
        │                     calls Registry.register_vow()
        │
        └── seal_vow()     → verifies partner auth, sets sealed=true
                              calls Registry.record_seal()
                              │
                              ▼
                        Registry Contract (WASM)
                        └── tracks all vows, seals, and combined stakes
```

## Contract Functions

```rust
// DualSeal Contract
propose_vow(proposer: Address, partner: Address, vow_text: String, stake_amount: i128) -> u64
seal_vow(vow_id: u64, signer: Address, partner_stake: i128)
get_vow(vow_id: u64) -> Vow
get_wallet_vows(wallet: Address) -> Vec<u64>
vow_count() -> u64
set_registry(admin: Address, registry_id: Address)
get_registry() -> Address

// Registry Contract (inter-contract)
register_vow(vow_id: u64, proposer: Address, partner: Address, stake_amount: i128)
record_seal(vow_id: u64, sealer: Address, partner_stake: i128)
is_registered(vow_id: u64) -> bool
get_entry(vow_id: u64) -> RegistryEntry
registry_count() -> u64
```
