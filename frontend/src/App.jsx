import { useState, useEffect, useCallback } from 'react'
import {
  connectWallet,
  proposeVow,
  sealVow,
  getVow,
  getVowCount,
  CONTRACT_ID,
} from './lib/stellar'

// ── Candle SVG component ───────────────────────────────────────────────────
function Candle({ delay = 0 }) {
  return (
    <div className="candle" style={{ animationDelay: `${delay}s` }}>
      <div className="flame">
        <div className="flame-inner" />
      </div>
      <div className="wick" />
      <div className="wax" />
    </div>
  )
}

// ── Ring SVG ───────────────────────────────────────────────────────────────
function RingIcon({ sealed }) {
  return (
    <svg width="64" height="64" viewBox="0 0 64 64" fill="none" className={`ring-icon ${sealed ? 'sealed' : ''}`}>
      <circle cx="32" cy="32" r="20" stroke="currentColor" strokeWidth="3" fill="none" />
      <circle cx="32" cy="32" r="14" stroke="currentColor" strokeWidth="1.5" fill="none" opacity="0.4" />
      {sealed && (
        <path d="M22 32 L29 39 L42 26" stroke="#d4af37" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round" />
      )}
    </svg>
  )
}

// ── Status badge ───────────────────────────────────────────────────────────
function StatusBadge({ sealed }) {
  return (
    <span className={`status-badge ${sealed ? 'status-sealed' : 'status-pending'}`}>
      {sealed ? '✦ Eternal' : '◌ Awaiting'}
    </span>
  )
}

// ── Vow Card ───────────────────────────────────────────────────────────────
function VowCard({ vow }) {
  const short = (addr) => addr ? `${addr.slice(0, 6)}…${addr.slice(-4)}` : ''
  const date = vow.timestamp
    ? new Date(Number(vow.timestamp) * 1000).toLocaleDateString('en-US', {
        year: 'numeric', month: 'long', day: 'numeric'
      })
    : '—'

  return (
    <div className={`vow-card ${vow.sealed ? 'vow-sealed' : 'vow-pending'}`}>
      <div className="vow-card-header">
        <span className="vow-id">VOW #{vow.id?.toString()}</span>
        <StatusBadge sealed={vow.sealed} />
      </div>
      <blockquote className="vow-text">"{vow.vow_text}"</blockquote>
      <div className="vow-parties">
        <div className="party">
          <span className="party-label">Proposer</span>
          <span className="party-addr">{short(vow.proposer?.toString())}</span>
          <span className="party-signed">{vow.proposer_signed ? '✓ Signed' : 'Pending'}</span>
        </div>
        <div className="party-divider">⟡</div>
        <div className="party">
          <span className="party-label">Partner</span>
          <span className="party-addr">{short(vow.partner?.toString())}</span>
          <span className="party-signed">{vow.partner_signed ? '✓ Sealed' : 'Awaiting'}</span>
        </div>
      </div>
      <div className="vow-date">{date}</div>
    </div>
  )
}

// ── Main App ───────────────────────────────────────────────────────────────
export default function App() {
  const [wallet, setWallet] = useState(null)
  const [tab, setTab] = useState('propose') // propose | seal | read
  const [loading, setLoading] = useState(false)
  const [status, setStatus] = useState(null) // { type: 'success'|'error', msg }
  const [vowCount, setVowCount] = useState('—')

  // Propose form
  const [partnerAddr, setPartnerAddr] = useState('')
  const [vowText, setVowText] = useState('')

  // Seal form
  const [sealVowId, setSealVowId] = useState('')

  // Read form
  const [readVowId, setReadVowId] = useState('')
  const [readVow, setReadVow] = useState(null)

  // Last result
  const [lastResult, setLastResult] = useState(null)

  useEffect(() => {
    getVowCount().then(setVowCount)
  }, [])

  const handleConnect = async () => {
    try {
      setLoading(true)
      const addr = await connectWallet()
      setWallet(addr)
      setStatus({ type: 'success', msg: 'Wallet connected ✓' })
    } catch (e) {
      setStatus({ type: 'error', msg: e.message })
    } finally {
      setLoading(false)
    }
  }

  const handlePropose = async (e) => {
    e.preventDefault()
    if (!wallet) return
    try {
      setLoading(true)
      setStatus({ type: 'info', msg: 'Awaiting wallet signature…' })
      const result = await proposeVow(wallet, partnerAddr, vowText)
      setLastResult(result)
      setStatus({
        type: 'success',
        msg: `Vow #${result.vowId} proposed on-chain`,
      })
      setVowCount(c => String(Number(c) + 1))
      setPartnerAddr('')
      setVowText('')
    } catch (e) {
      setStatus({ type: 'error', msg: e.message })
    } finally {
      setLoading(false)
    }
  }

  const handleSeal = async (e) => {
    e.preventDefault()
    if (!wallet) return
    try {
      setLoading(true)
      setStatus({ type: 'info', msg: 'Sealing vow on-chain…' })
      const result = await sealVow(sealVowId, wallet)
      setLastResult(result)
      setStatus({ type: 'success', msg: `Vow #${sealVowId} sealed forever ✦` })
      setSealVowId('')
    } catch (e) {
      setStatus({ type: 'error', msg: e.message })
    } finally {
      setLoading(false)
    }
  }

  const handleRead = async (e) => {
    e.preventDefault()
    try {
      setLoading(true)
      const vow = await getVow(readVowId)
      setReadVow(vow)
      setStatus(null)
    } catch (e) {
      setStatus({ type: 'error', msg: 'Vow not found' })
      setReadVow(null)
    } finally {
      setLoading(false)
    }
  }

  const short = (addr) => addr ? `${addr.slice(0, 8)}…${addr.slice(-6)}` : ''

  return (
    <>
      <div className="bg-texture" />

      {/* Candles */}
      <div className="candles-left">
        <Candle delay={0} />
        <Candle delay={0.3} />
        <Candle delay={0.7} />
      </div>
      <div className="candles-right">
        <Candle delay={0.5} />
        <Candle delay={0.1} />
        <Candle delay={0.9} />
      </div>

      <div className="app">
        {/* Header */}
        <header className="header">
          <div className="header-ornament">— ✦ ✦ ✦ —</div>
          <h1 className="title">ChainVow</h1>
          <p className="subtitle">Eternal commitments, sealed on the Stellar blockchain</p>
          <div className="header-ornament">— ✦ ✦ ✦ —</div>

          <div className="header-meta">
            <div className="meta-item">
              <span className="meta-label">Vows on-chain</span>
              <span className="meta-value">{vowCount}</span>
            </div>
            <div className="meta-divider" />
            <div className="meta-item">
              <span className="meta-label">Network</span>
              <span className="meta-value">Stellar Testnet</span>
            </div>
            <div className="meta-divider" />
            <div className="meta-item">
              <span className="meta-label">Contract</span>
              <a
                className="meta-value meta-link"
                href={`https://stellar.expert/explorer/testnet/contract/${CONTRACT_ID}`}
                target="_blank"
                rel="noreferrer"
              >
                {CONTRACT_ID ? short(CONTRACT_ID) : 'Not deployed'}
              </a>
            </div>
          </div>
        </header>

        {/* Wallet section */}
        <div className="wallet-section">
          {wallet ? (
            <div className="wallet-connected">
              <div className="wallet-dot" />
              <span>{short(wallet)}</span>
            </div>
          ) : (
            <button className="btn-connect" onClick={handleConnect} disabled={loading}>
              {loading ? 'Connecting…' : 'Connect Freighter'}
            </button>
          )}
        </div>

        {/* Status */}
        {status && (
          <div className={`status-msg status-${status.type}`}>
            {status.msg}
            {lastResult && (
              <div className="tx-result">
                <a
                  href={`https://stellar.expert/explorer/testnet/tx/${lastResult.txHash}`}
                  target="_blank"
                  rel="noreferrer"
                  className="tx-link"
                >
                  View on Stellar Expert →
                </a>
              </div>
            )}
          </div>
        )}

        {/* Main Panel */}
        <div className="panel">
          <div className="tabs">
            {[
              { id: 'propose', label: 'Propose Vow' },
              { id: 'seal', label: 'Seal Vow' },
              { id: 'read', label: 'Read Vow' },
            ].map(t => (
              <button
                key={t.id}
                className={`tab ${tab === t.id ? 'tab-active' : ''}`}
                onClick={() => setTab(t.id)}
              >
                {t.label}
              </button>
            ))}
          </div>

          {/* Propose */}
          {tab === 'propose' && (
            <div className="form-section">
              <div className="form-icon"><RingIcon sealed={false} /></div>
              <p className="form-desc">
                Write your commitment. Your partner will receive a vow ID to seal it forever.
              </p>
              <form onSubmit={handlePropose} className="form">
                <div className="field">
                  <label>Partner's Stellar Address</label>
                  <input
                    value={partnerAddr}
                    onChange={e => setPartnerAddr(e.target.value)}
                    placeholder="G…"
                    required
                    disabled={!wallet || loading}
                  />
                </div>
                <div className="field">
                  <label>Your Vow</label>
                  <textarea
                    value={vowText}
                    onChange={e => setVowText(e.target.value)}
                    placeholder="Write your commitment here. It will be stored immutably on the Stellar blockchain, forever."
                    rows={5}
                    required
                    disabled={!wallet || loading}
                    maxLength={500}
                  />
                  <span className="char-count">{vowText.length}/500</span>
                </div>
                <button
                  type="submit"
                  className="btn-primary"
                  disabled={!wallet || loading || !partnerAddr || !vowText}
                >
                  {loading ? 'Proposing…' : 'Propose Vow on Stellar'}
                </button>
                {!wallet && (
                  <p className="form-hint">Connect your Freighter wallet to continue</p>
                )}
              </form>
            </div>
          )}

          {/* Seal */}
          {tab === 'seal' && (
            <div className="form-section">
              <div className="form-icon"><RingIcon sealed={true} /></div>
              <p className="form-desc">
                Accept your partner's vow. This seals it on-chain — immutable, forever.
              </p>
              <form onSubmit={handleSeal} className="form">
                <div className="field">
                  <label>Vow ID</label>
                  <input
                    value={sealVowId}
                    onChange={e => setSealVowId(e.target.value)}
                    placeholder="Enter vow number, e.g. 1"
                    required
                    type="number"
                    min="1"
                    disabled={!wallet || loading}
                  />
                </div>
                <button
                  type="submit"
                  className="btn-primary btn-seal"
                  disabled={!wallet || loading || !sealVowId}
                >
                  {loading ? 'Sealing…' : '✦ Seal This Vow'}
                </button>
              </form>
            </div>
          )}

          {/* Read */}
          {tab === 'read' && (
            <div className="form-section">
              <p className="form-desc">
                Read any vow from the blockchain by its ID.
              </p>
              <form onSubmit={handleRead} className="form">
                <div className="field">
                  <label>Vow ID</label>
                  <input
                    value={readVowId}
                    onChange={e => setReadVowId(e.target.value)}
                    placeholder="e.g. 1"
                    required
                    type="number"
                    min="1"
                    disabled={loading}
                  />
                </div>
                <button
                  type="submit"
                  className="btn-primary"
                  disabled={loading || !readVowId}
                >
                  {loading ? 'Reading…' : 'Read from Chain'}
                </button>
              </form>
              {readVow && <VowCard vow={readVow} />}
            </div>
          )}
        </div>

        {/* Footer */}
        <footer className="footer">
          <p>CHAINVOW — ETERNAL COMMITMENTS ON STELLAR</p>
          <p className="footer-contract">
            Contract ID:{' '}
            <a
              href={`https://stellar.expert/explorer/testnet/contract/${CONTRACT_ID}`}
              target="_blank"
              rel="noreferrer"
            >
              {CONTRACT_ID || 'Deploy to activate'}
            </a>
          </p>
        </footer>
      </div>
    </>
  )
}
