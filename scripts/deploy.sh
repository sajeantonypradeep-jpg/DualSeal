#!/usr/bin/env bash
# ChainVow — Full deploy script
# Prerequisites: Rust + wasm32 target, stellar-cli, funded testnet account
# Run: chmod +x deploy.sh && ./deploy.sh

set -e
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'

echo -e "${CYAN}"
echo "  ╔═══════════════════════════════════╗"
echo "  ║     ChainVow — Deploy Script      ║"
echo "  ╚═══════════════════════════════════╝"
echo -e "${NC}"

# ── 1. Setup testnet identity ──────────────────────────────────────────────
echo -e "${YELLOW}[1/6] Setting up testnet identity...${NC}"
stellar keys generate --global deployer --network testnet 2>/dev/null || true
stellar keys fund deployer --network testnet
DEPLOYER_ADDR=$(stellar keys address deployer)
echo -e "${GREEN}✓ Deployer: ${DEPLOYER_ADDR}${NC}"

# ── 2. Build contract ──────────────────────────────────────────────────────
echo -e "${YELLOW}[2/6] Building Soroban contract...${NC}"
cd contract
cargo build --target wasm32-unknown-unknown --release
WASM_PATH="target/wasm32-unknown-unknown/release/chainvow.wasm"
echo -e "${GREEN}✓ Built: ${WASM_PATH}${NC}"
cd ..

# ── 3. Upload WASM to testnet ──────────────────────────────────────────────
echo -e "${YELLOW}[3/6] Uploading WASM to Stellar testnet...${NC}"
WASM_HASH=$(stellar contract upload \
  --network testnet \
  --source deployer \
  --wasm contract/${WASM_PATH})
echo -e "${GREEN}✓ WASM Hash: ${WASM_HASH}${NC}"

# ── 4. Deploy contract ─────────────────────────────────────────────────────
echo -e "${YELLOW}[4/6] Deploying contract instance...${NC}"
CONTRACT_ID=$(stellar contract deploy \
  --network testnet \
  --source deployer \
  --wasm-hash ${WASM_HASH})
echo -e "${GREEN}✓ CONTRACT_ID: ${CONTRACT_ID}${NC}"

# ── 5. Invoke contract (create proof tx) ───────────────────────────────────
echo -e "${YELLOW}[5/6] Creating proof transaction (propose_vow)...${NC}"

# Generate a second test wallet as partner
stellar keys generate --global partner --network testnet 2>/dev/null || true
PARTNER_ADDR=$(stellar keys address partner)

TX_HASH=$(stellar contract invoke \
  --network testnet \
  --source deployer \
  --id ${CONTRACT_ID} \
  -- \
  propose_vow \
  --proposer ${DEPLOYER_ADDR} \
  --partner ${PARTNER_ADDR} \
  --vow_text "\"We commit to building the future together, on-chain and off.\"" \
  2>&1 | grep -oP '[0-9a-f]{64}' | head -1)

echo -e "${GREEN}✓ TX Hash: ${TX_HASH}${NC}"

# ── 6. Write .env for frontend ─────────────────────────────────────────────
echo -e "${YELLOW}[6/6] Writing frontend environment...${NC}"
cat > frontend/.env << EOF
VITE_CONTRACT_ID=${CONTRACT_ID}
VITE_NETWORK_PASSPHRASE=Test SDF Network ; September 2015
VITE_SOROBAN_RPC_URL=https://soroban-testnet.stellar.org
VITE_HORIZON_URL=https://horizon-testnet.stellar.org
EOF

# ── Summary ────────────────────────────────────────────────────────────────
echo ""
echo -e "${CYAN}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║                  DEPLOYMENT COMPLETE                    ║${NC}"
echo -e "${CYAN}╠══════════════════════════════════════════════════════════╣${NC}"
echo -e "${CYAN}║${NC} Contract ID : ${GREEN}${CONTRACT_ID}${NC}"
echo -e "${CYAN}║${NC} TX Hash     : ${GREEN}${TX_HASH}${NC}"
echo -e "${CYAN}║${NC} Explorer    : ${GREEN}https://stellar.expert/explorer/testnet/contract/${CONTRACT_ID}${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "Next: ${YELLOW}cd frontend && npm install && npm run dev${NC}"
