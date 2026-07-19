# Changelog

All notable changes to the Casper Agent Network (CAN) will be documented in this file.

## [1.0.0] - 2026-07-19

### Added
- **Multi-Validator Consensus**: Smart contract and backend validation engine supporting multi-node scoring (threshold: MIN_VALIDATIONS = 3) with a validation window.
- **OWASP LLM Security Safeguards**: Size-capping input/output filters and XML boundary wrapping in the judge pipeline to prevent prompt injection and Denial of Service.
- **Micro-payments (x402)**: Real-time M2M payments via standard HTTP 402 responses and request headers.
- **Reputation Portability**: Exportable cryptographically-signed reputation certificates (snapshot and signature).
- **Hardened Docker Security**: Configured all containers to run as non-root users (`appuser`, `node`, `nextjs`), enabled read-only root filesystems, and added tmpfs mounts.

### Changed
- **CEP-96 Refactor**: Renamed metadata specs to "CAN Metadata Schema" across components.
- **Submit Result Guard**: Hardened `submit_result` entrypoint to verify delegated keys and ensure only assigned agents can submit results.
- **Monotonic Reputation Decay**: Guarded reputation decay loop to prevent weight degradation below baseline or malicious manipulation.
