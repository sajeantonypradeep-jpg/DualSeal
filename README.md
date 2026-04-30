# DualSeal Protocol ✦

**Two signatures. One irreversible truth.**

![CI](https://github.com/sajeantonypradeep-jpg/DualSeal/actions/workflows/ci.yml/badge.svg)

---

## 🚀 Overview

DualSeal Protocol is a Soroban-based smart contract system that enables two Stellar wallet holders to create immutable, dual-signed commitments on-chain.

A proposer defines a vow and signs it. The counterparty seals it. Once sealed, the commitment becomes permanently recorded, tamper-proof, and publicly verifiable.

---

## ✅ Status

* CI/CD: Passing
* Network: Stellar Testnet
* Deployment: Live

---

## ⚙️ Core Features

* **Dual-Signature Workflow** — two independent wallet approvals
* **Immutable On-Chain Storage** — vows cannot be altered once sealed
* **Inter-Contract Communication** — registry/verifier contract interaction
* **Stake Mechanism** — optional value commitment tied to vows
* **Wallet Authentication** — powered by Freighter signatures
* **Deterministic Execution** — enforced via Soroban smart contracts

---

## 🔍 How It Works

1. A user proposes a vow by submitting:

   * Partner address
   * Commitment text
   * Optional stake

2. The contract:

   * Stores the vow
   * Marks proposer as signed
   * Emits an event

3. The partner reviews and seals the vow:

   * Signature is verified
   * State is locked permanently

4. Once sealed:

   * The vow becomes immutable
   * Both parties are cryptographically bound

---

## 🔗 Inter-Contract Interaction

DualSeal integrates with a secondary contract (Registry/Verifier) to:

* Record vow lifecycle events
* Validate interactions across contracts
* Demonstrate composability within Soroban

This showcases modular smart contract design and cross-contract communication.

---

## 🏗️ Architecture

**Smart Contract Layer**
Soroban contracts enforce business logic, validation, and state transitions.

**Client Layer**
React + Vite frontend handles UI, transaction construction, and state display.

**Wallet Layer**
Freighter wallet signs all state-changing transactions.

**Infrastructure Layer**
Stellar Testnet + Soroban RPC handle execution and communication.

---

## 📜 Smart Contract Functions

```rust
propose_vow(proposer: Address, partner: Address, vow_text: String) -> u64
// Creates a new vow and records proposer signature

seal_vow(vow_id: u64, signer: Address)
// Finalizes the vow and locks it permanently

get_vow(vow_id: u64) -> Vow
// Retrieves vow details

get_wallet_vows(wallet: Address) -> Vec<u64>
// Lists all vows linked to a wallet

vow_count() -> u64
// Returns total number of vows
```

---

## 🔄 CI/CD

This project uses GitHub Actions for continuous integration.

On every push or pull request:

* Smart contract is built (Rust + WASM)
* Unit tests are executed
* Frontend is built using Vite

This ensures:

* Code reliability
* Build consistency
* Production readiness

---

## 🧰 Tech Stack

* **Smart Contract:** Rust + Soroban SDK
* **Blockchain:** Stellar Testnet
* **Frontend:** React + Vite
* **Wallet:** Freighter
* **RPC:** Soroban RPC
* **CI/CD:** GitHub Actions

---

## 🌐 Live Links

* **Frontend:** https://dual-seal.vercel.app
* **Contract:** https://stellar.expert/explorer/testnet/contract/CBIVYOVF66XZYUAF3YG6NKJI4R366HLUTHGL2C3WMBSO5HPVO5FNUBZU

---

## 🛠️ Local Setup

### Prerequisites

* Rust + `wasm32-unknown-unknown` target
* Stellar CLI v25+
* Node.js 20+
* Freighter wallet extension

---

### Run Locally

```bash
# Deploy contract
chmod +x scripts/deploy.sh && ./scripts/deploy.sh

# Run frontend
cd frontend
npm install
npm run dev
```

---

## 📁 Project Structure

```
DualSeal/
├── contract/     # Soroban smart contract
├── frontend/     # React frontend
├── scripts/      # Deployment scripts
├── tests/        # Contract tests
├── .github/      # CI/CD workflows
└── README.md
```

---

## 🔐 Security Considerations

* Only the designated partner can seal a vow
* Double-sealing is prevented at the contract level
* All state transitions are validated and deterministic
* Inputs are validated before execution

---

## 🚧 Future Improvements

* Multi-party commitments (more than two participants)
* Expiry conditions for vows
* Token-based incentives and penalties
* Off-chain indexing for faster querying

---

## 🎯 Why This Project Matters

DualSeal demonstrates how real-world agreements can be translated into **verifiable, trustless digital commitments**.

It highlights:

* Secure multi-party authorization
* Transparent state transitions
* Immutable on-chain records

---

## 📄 License

MIT License
