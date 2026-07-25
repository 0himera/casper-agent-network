# Casper Agent Network — Bittensor-Class Extensions Architecture

## 1. Executive Summary

Casper Agent Network incorporates Bittensor-class subnetwork consensus mechanics directly onto the Casper Network. This document outlines the technical architecture for validator weights commit/reveal epochs, Yuma-style consensus, and incentive mechanisms.

## 2. Commit-Reveal Epoch Mechanism

To prevent front-running and copy-cat scoring among validator nodes, validator weight submissions follow a two-phase commit-reveal epoch model:

1. **Epoch Timing**:
   - Total Epoch Duration: **15 minutes** (900,000 ms).
   - **Commit Window**: First 10 minutes (0 to 600,000 ms).
   - **Reveal Window**: Last 5 minutes (600,000 to 900,000 ms).

2. **Commit Phase**:
   - Validator computes evaluation scores for subnetwork agents off-chain.
   - Submits `commit_hash = sha256(weights_json + salt)` to the contract via `commit_weights(commit_hash)`.

3. **Reveal Phase**:
   - During the reveal window, validator submits `reveal_weights(weights_json, salt)`.
   - Contract verifies sha256 match against `commit_hash` before accepting scores.

## 3. Yuma-Style Consensus & Slashing

- **Median Calculation**: Consensus score for each task/subnetwork is calculated using the median of all revealed validator scores.
- **Deviation Tolerance**: `DEVIATION_TOLERANCE = 10` score points.
- **Slashing Penalty**: Validators deviating by more than 10 points are penalized at `500 bps (5%)` per 10 points of deviation, up to 100% stake slash.
- **Stake-Weighted Rewards**: Non-deviating validators receive stake-weighted distribution from protocol fees and slash penalties.
