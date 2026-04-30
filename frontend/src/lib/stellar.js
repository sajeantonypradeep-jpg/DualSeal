import * as StellarSdk from '@stellar/stellar-sdk'
import { isConnected, getPublicKey, signTransaction } from '@stellar/freighter-api'

const CONTRACT_ID = (import.meta.env.VITE_CONTRACT_ID || '').trim()
const NETWORK_PASSPHRASE = (import.meta.env.VITE_NETWORK_PASSPHRASE || 'Test SDF Network ; September 2015').trim()
const SOROBAN_RPC_URL = (import.meta.env.VITE_SOROBAN_RPC_URL || 'https://soroban-testnet.stellar.org').trim()

export const rpc = new StellarSdk.rpc.Server(SOROBAN_RPC_URL)

// ── Wallet ────────────────────────────────────────────────────────────────
export async function connectWallet() {
  if (!(await isConnected())) throw new Error('Freighter not installed.')
  return await getPublicKey()
}

// ── Contract helpers ──────────────────────────────────────────────────────
function getContract(publicKey) {
  const account = new StellarSdk.Account(publicKey, '0')
  return {
    contract: new StellarSdk.Contract(CONTRACT_ID),
    account,
  }
}

async function simulateAndSend(tx, publicKey) {
  const simResult = await rpc.simulateTransaction(tx)
  if (StellarSdk.rpc.Api.isSimulationError(simResult)) {
    throw new Error(`Simulation failed: ${simResult.error}`)
  }

  const preparedTx = StellarSdk.rpc.assembleTransaction(tx, simResult).build()
  const xdr = preparedTx.toXDR()

  const signedXdr = await signTransaction(xdr, { networkPassphrase: NETWORK_PASSPHRASE, network: 'TESTNET' })

  const txFromXDR = StellarSdk.TransactionBuilder.fromXDR(
    signedXdr,
    NETWORK_PASSPHRASE
  )

  const sent = await rpc.sendTransaction(txFromXDR)
  return sent
}

async function pollTx(hash) {
  let attempts = 0
  while (attempts < 30) {
    const result = await rpc.getTransaction(hash)
    if (result.status === 'SUCCESS') return result
    if (result.status === 'FAILED') throw new Error('Transaction failed on-chain')
    await new Promise(r => setTimeout(r, 2000))
    attempts++
  }
  throw new Error('Transaction timed out')
}

// ── Propose Vow ───────────────────────────────────────────────────────────
export async function proposeVow(proposerAddress, partnerAddress, vowText) {
  const sourceAccount = await rpc.getAccount(proposerAddress)
  const { contract } = getContract(proposerAddress)

  const tx = new StellarSdk.TransactionBuilder(sourceAccount, {
    fee: StellarSdk.BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(
      contract.call(
        'propose_vow',
        StellarSdk.Address.fromString(proposerAddress).toScVal(),
        StellarSdk.Address.fromString(partnerAddress).toScVal(),
        StellarSdk.xdr.ScVal.scvString(vowText)
      )
    )
    .setTimeout(30)
    .build()

  const sendResult = await simulateAndSend(tx, proposerAddress)
  const finalResult = await pollTx(sendResult.hash)

  // Extract returned vow ID (u64)
  const retVal = finalResult.returnValue
  const vowId = StellarSdk.scValToNative(retVal)
  return { vowId: vowId.toString(), txHash: sendResult.hash }
}

// ── Seal Vow ──────────────────────────────────────────────────────────────
export async function sealVow(vowId, signerAddress) {
  const sourceAccount = await rpc.getAccount(signerAddress)
  const { contract } = getContract(signerAddress)

  const tx = new StellarSdk.TransactionBuilder(sourceAccount, {
    fee: StellarSdk.BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(
      contract.call(
        'seal_vow',
        StellarSdk.xdr.ScVal.scvU64(
          new StellarSdk.xdr.Uint64(BigInt(vowId))
        ),
        StellarSdk.Address.fromString(signerAddress).toScVal()
      )
    )
    .setTimeout(30)
    .build()

  const sendResult = await simulateAndSend(tx, signerAddress)
  await pollTx(sendResult.hash)
  return { txHash: sendResult.hash }
}

// ── Get Vow ───────────────────────────────────────────────────────────────
export async function getVow(vowId) {
  const result = await rpc.simulateTransaction(
    new StellarSdk.TransactionBuilder(
      new StellarSdk.Account(
        'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN',
        '0'
      ),
      {
        fee: StellarSdk.BASE_FEE,
        networkPassphrase: NETWORK_PASSPHRASE,
      }
    )
      .addOperation(
        new StellarSdk.Contract(CONTRACT_ID).call(
          'get_vow',
          StellarSdk.xdr.ScVal.scvU64(
            new StellarSdk.xdr.Uint64(BigInt(vowId))
          )
        )
      )
      .setTimeout(30)
      .build()
  )

  const native = StellarSdk.scValToNative(result.result.retval)
  return native
}

// ── Vow Count ─────────────────────────────────────────────────────────────
export async function getVowCount() {
  try {
    const result = await rpc.simulateTransaction(
      new StellarSdk.TransactionBuilder(
        new StellarSdk.Account(
          'GAAZI4TCR3TY5OJHCTJC2A4QSY6CJWJH5IAJTGKIN2ER7LBNVKOCCWN',
          '0'
        ),
        {
          fee: StellarSdk.BASE_FEE,
          networkPassphrase: NETWORK_PASSPHRASE,
        }
      )
        .addOperation(
          new StellarSdk.Contract(CONTRACT_ID).call('vow_count')
        )
        .setTimeout(30)
        .build()
    )
    return StellarSdk.scValToNative(result.result.retval).toString()
  } catch {
    return '0'
  }
}

export { CONTRACT_ID, NETWORK_PASSPHRASE }



