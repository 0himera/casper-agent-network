# Contributing Guidelines

Thank you for your interest in contributing to the Casper Agent Network (CAN)! This document outlines the process and standards for contributing code, tests, and documentation.

## Development Workflow

1. **Fork and Branch**: Create a feature branch from `master` (e.g., `feat/add-new-metric`).
2. **Implement Changes**: Add your code or documentation changes.
3. **Write Tests**: Ensure every new feature has accompanying unit and integration tests.
4. **Lint and Style**: Run standard formatters:
   - For Rust code: `cargo fmt` and `cargo clippy`.
   - For TypeScript/React code: `npm run lint`.
5. **Verify Compiles**: Run all tests before submitting:
   - Contract: `cargo test` in `app/smart-contract`.
   - Backend: `cargo test` in `app/backend`.
6. **Submit PR**: Open a Pull Request referencing the issue you are addressing.

## Coding Standards

### Rust Smart Contracts (Odra)
- Enforce strict Access Control Lists (ACL) on state modification entry points (using `assert_admin()`).
- Always check math bounds and avoid integer overflow (utilize safe math libraries or `checked_*` methods).
- Events must be emitted for every state transition to allow event-handler streaming updates.

### TypeScript / Node.js BFF
- Always authenticate API requests. All admin actions must require a valid `adminToken` matching the configured `process.env.MCP_ADMIN_TOKEN`.
- Avoid hardcoding secrets. Load private keys and credentials via environment variables or secret managers.
- Enforce strict rate-limiting for write operations.
