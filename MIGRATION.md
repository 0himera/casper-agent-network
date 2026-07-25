# Migration and Rollback Strategy

This document details the transition strategy from Casper Agent Network v1 to v2, including contract upgrades and backend rollback procedures.

## 1. On-Chain Smart Contract Upgrade

The Agent Network contract uses the **Odra** framework. 
Because Odra smart contracts are packaged on Casper, we deploy the upgraded contract v2 to the network and declare the previous v1 package deprecated.

### Steps:
1. **Build & Deploy v2**:
   ```bash
   cd app/smart-contract
   cargo odra deploy --network livenet
   ```
2. **Deprecate Beta v1**:
   Mark the old contract package hash on our indexers and clients as `deprecated beta`. The frontend dynamically prompts operators to migrate active agents to the new contract package.
3. **Admin Key Rotation**:
   Upon deployment, run the ownership transfer script from the admin shell to ensure secure key transition:
   ```bash
   # In the CLI or MCP tool:
   transfer_ownership(new_admin_pubkey)
   # From the new admin's key:
   accept_ownership()
   ```

---

## 2. Rollback Strategy

In case of critical failures post-deployment, follow the instructions below to restore the system to a clean and stable state.

### Backend Rollback:
If the database schema or validator engine fails, roll back the backend image to the stable pre-upgrade state:
1. **Revert Git Repository**:
   ```bash
   git checkout 9e1bf10
   ```
2. **Re-deploy Docker Containers**:
   ```bash
   docker compose down
   docker compose up -d --build
   ```

### Database Rollback:
To revert database migrations if needed:
- Roll back to the previous stable snapshot:
  ```bash
  mysql -u root -p deagentnet < pre_migration_backup.sql
  ```
