# Docker MySQL host port workaround

**Status:** temporary — active while local Homebrew MySQL binds `3306`.

## What changed

- `docker-compose.yaml`: `mysql.ports` is `3307:3306` (was `3306:3306`)
- `backend/.env`: `DATABASE_URL=mysql://deagentnet:passw0rd@localhost:3307/deagentnet`

Internal compose services (`backend`, `mcp`, `event-handler`) still use `mysql:3306` inside the Docker network — only the **host** mapping differs.

## Running backend tests with Docker MySQL

`cargo test` does **not** load `backend/.env`. Export `DATABASE_URL` explicitly.

**Unit tests only (no MySQL required):**
```bash
cd backend
cargo test --lib
```
Expected: `27 passed`, `5 ignored`.

**E2 DB-backed tests (`#[ignore]` until `--ignored`):**
```bash
cd backend
export DATABASE_URL='mysql://deagentnet:passw0rd@localhost:3307/deagentnet'
cargo test --lib db_ -- --ignored --test-threads=1
```
Expected: `5 passed`.

Use `--test-threads=1` — parallel runs can flake because E2 fixtures share
`exam_templates` and `agents` rows across tests.

**Full suite (unit + DB):**
```bash
cd backend
export DATABASE_URL='mysql://deagentnet:passw0rd@localhost:3307/deagentnet'
cargo test --lib -- --test-threads=1
```
Expected: `32 passed`.

If you see `Access denied for user 'root'@'localhost'`, you forgot `DATABASE_URL`
(the test fallback is `root:password@127.0.0.1:3306`, not the Docker credentials).

## Revert when port 3306 is free again

1. In `docker-compose.yaml`, restore:
   ```yaml
   ports:
     - "3306:3306"
   ```
2. In `backend/.env`, restore Homebrew URL if needed:
   ```env
   DATABASE_URL=mysql://root@localhost:3306/casper_agent_network
   ```
3. Recreate the container:
   ```bash
   docker rm -f agent-network-mysql
   docker compose up -d mysql
   ```
