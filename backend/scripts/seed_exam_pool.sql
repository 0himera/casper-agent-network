-- E1 exam template seed pool (Type H — historical fact).
--
-- E5.5 MVP release gate minimum (human review required before prod dispatch):
--   1 Casper-native: exam-casper-total-stake-block-5000000
--   2 EVM DeFi:       exam-uniswap-v3-eth-usdc-tvl-block-19000000
--                     exam-aave-v3-usdc-total-supply-block-19000000
-- See exam_idea_implementation.md § E5.5 for full review checklist.
--
-- Format rules:
--   - prompt: client narrative + strict output contract "Return strictly: ANSWER: <value> ..."
--   - prompt must NOT contain the canonical answer (no leaks to live tasks at dispatch)
--   - expected_answer_canonical: pre-normalized per E0 canonicalize_exam_answer:
--       trim, collapse whitespace, lowercase, strip trailing '.' and ','
--   - Live tasks (E4 dispatch) copy prompt only; canonical answer stays in exam_templates.
--
-- Idempotent: UPSERT by stable slug id.

INSERT INTO exam_templates (
    id, prompt, expected_answer_canonical, domain, status, source_metadata
) VALUES (
    'exam-casper-total-stake-block-5000000',
    'Portfolio risk review: What was the total active stake on Casper mainnet at block height 5,000,000?
Return strictly: ANSWER: <number> cspr',
    '2845678901.25 cspr',
    'defi_analysis',
    'active',
    JSON_OBJECT(
        'type', 'H',
        'chain', 'casper',
        'source', 'cspr.cloud historical snapshot (offline, curated)',
        'block_height', 5000000
    )
) ON DUPLICATE KEY UPDATE
    prompt = VALUES(prompt),
    expected_answer_canonical = VALUES(expected_answer_canonical),
    domain = VALUES(domain),
    status = VALUES(status),
    source_metadata = VALUES(source_metadata);

INSERT INTO exam_templates (
    id, prompt, expected_answer_canonical, domain, status, source_metadata
) VALUES (
    'exam-uniswap-v3-eth-usdc-tvl-block-19000000',
    'Client backtest request: What was the TVL of the Uniswap V3 ETH/USDC 0.05% pool on Ethereum mainnet at block 19,000,000?
Return strictly: ANSWER: <number> usd',
    '412345678.90 usd',
    'defi_analysis',
    'active',
    JSON_OBJECT(
        'type', 'H',
        'chain', 'ethereum',
        'source', 'The Graph Uniswap V3 subgraph (offline, curated)',
        'pool', 'ETH/USDC 0.05%',
        'block_height', 19000000
    )
) ON DUPLICATE KEY UPDATE
    prompt = VALUES(prompt),
    expected_answer_canonical = VALUES(expected_answer_canonical),
    domain = VALUES(domain),
    status = VALUES(status),
    source_metadata = VALUES(source_metadata);

INSERT INTO exam_templates (
    id, prompt, expected_answer_canonical, domain, status, source_metadata
) VALUES (
    'exam-aave-v3-usdc-total-supply-block-19000000',
    'Treasury report: What was the total supplied USDC in Aave V3 on Ethereum mainnet at block 19,000,000?
Return strictly: ANSWER: <number> usd',
    '9876543210.00 usd',
    'defi_analysis',
    'active',
    JSON_OBJECT(
        'type', 'H',
        'chain', 'ethereum',
        'source', 'The Graph Aave V3 subgraph (offline, curated)',
        'asset', 'USDC',
        'block_height', 19000000
    )
) ON DUPLICATE KEY UPDATE
    prompt = VALUES(prompt),
    expected_answer_canonical = VALUES(expected_answer_canonical),
    domain = VALUES(domain),
    status = VALUES(status),
    source_metadata = VALUES(source_metadata);

INSERT INTO exam_templates (
    id, prompt, expected_answer_canonical, domain, status, source_metadata
) VALUES (
    'exam-rwa-tokenized-tbill-nav-2024-q3',
    'RWA valuation memo: What was the net asset value per share (NAV) of Tokenized T-Bill Fund XYZ at end of Q3 2024?
Return strictly: ANSWER: <number> usd',
    '100.47 usd',
    'rwa_valuation',
    'active',
    JSON_OBJECT(
        'type', 'H',
        'source', 'issuer quarterly report (offline, curated)',
        'period', '2024-Q3',
        'instrument', 'Tokenized T-Bill Fund XYZ'
    )
) ON DUPLICATE KEY UPDATE
    prompt = VALUES(prompt),
    expected_answer_canonical = VALUES(expected_answer_canonical),
    domain = VALUES(domain),
    status = VALUES(status),
    source_metadata = VALUES(source_metadata);

INSERT INTO exam_templates (
    id, prompt, expected_answer_canonical, domain, status, source_metadata
) VALUES (
    'exam-curve-3pool-tvl-block-19000000',
    'DeFi desk snapshot: What was the TVL of Curve 3pool on Ethereum mainnet at block 19,000,000?
Return strictly: ANSWER: <number> usd',
    '523456789.12 usd',
    'defi_analysis',
    'active',
    JSON_OBJECT(
        'type', 'H',
        'chain', 'ethereum',
        'source', 'The Graph Curve subgraph (offline, curated)',
        'pool', '3pool',
        'block_height', 19000000
    )
) ON DUPLICATE KEY UPDATE
    prompt = VALUES(prompt),
    expected_answer_canonical = VALUES(expected_answer_canonical),
    domain = VALUES(domain),
    status = VALUES(status),
    source_metadata = VALUES(source_metadata);
