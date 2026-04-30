#!/usr/bin/env bash
# Creates GitHub repo and pushes + deploys to Vercel
# Prerequisites: gh CLI (logged in), vercel CLI (logged in)

set -e
CYAN='\033[0;36m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'

REPO_NAME="chainvow"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo -e "${CYAN}[1/4] Creating GitHub repo: ${REPO_NAME}${NC}"
gh repo create "${REPO_NAME}" \
  --public \
  --description "ChainVow — Eternal commitments sealed on the Stellar blockchain" \
  --source "${ROOT_DIR}" \
  --remote origin \
  --push

echo -e "${GREEN}✓ Repo: https://github.com/$(gh api user -q .login)/${REPO_NAME}${NC}"

echo -e "${CYAN}[2/4] Setting Vercel env secrets...${NC}"
CONTRACT_ID=$(grep VITE_CONTRACT_ID "${ROOT_DIR}/frontend/.env" | cut -d= -f2)
gh secret set VITE_CONTRACT_ID --body "${CONTRACT_ID}" --repo "$(gh api user -q .login)/${REPO_NAME}"

echo -e "${CYAN}[3/4] Deploying frontend to Vercel...${NC}"
cd "${ROOT_DIR}/frontend"
vercel --prod --yes

DEPLOYMENT_URL=$(vercel ls --json 2>/dev/null | node -e "
  const d = require('fs').readFileSync('/dev/stdin','utf8');
  const arr = JSON.parse(d);
  console.log(arr[0]?.url || '');
")

echo ""
echo -e "${GREEN}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                  PUBLISHED LINKS                        ║${NC}"
echo -e "${GREEN}╠══════════════════════════════════════════════════════════╣${NC}"
echo -e "${GREEN}║${NC} GitHub  : https://github.com/$(gh api user -q .login)/${REPO_NAME}"
echo -e "${GREEN}║${NC} Frontend: https://${DEPLOYMENT_URL}"
echo -e "${GREEN}║${NC} Contract: https://stellar.expert/explorer/testnet/contract/${CONTRACT_ID}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════════╝${NC}"
