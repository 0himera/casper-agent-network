# Casper Agent Network - Server

The TS server module consists of:
1. **Event Listener** - Streams live contract events from the Casper blockchain via CSPR.cloud WebSockets and indexes them directly into the shared MySQL database.
2. **MCP Server** - Programmatic interface exposing Model Context Protocol (MCP) tools for agent discovery, transaction building, and autonomous integrations.

## Architecture

```
Casper Blockchain → CSPR.cloud → Event Listener → MySQL Database ◄─ Rust Backend (Axum, :8080)
                                                    ▲
                                                    │ (Direct SQL, SSE on :4000)
                                             MCP Server (TS, SSE / Stdio)
```

The Event Listener subscribes to smart contract events via CSPR.cloud's real-time streaming API, processes them, and stores structured data in MySQL. The Rust Axum Backend acts as the REST API and execution manager for the web client. The MCP Server runs in SSE mode by default (port 4000) for autonomous agent integrations, with Stdio mode also available for local editor use.

## Prerequisites

- **Node.js**: Version 20.12.0 or higher
- **npm**: Version 8.x or higher
- **MySQL**: Version 8.0 or higher
- **CSPR.cloud API Key**: Obtain from [CSPR.build Console](https://console.cspr.build)

### Database Setup

You can run MySQL locally or use Docker:

**Option A: Docker (Recommended for Development)**
```bash
docker compose -f ../docker-compose.yaml up -d mysql
```

**Option B: Local MySQL Installation**
- Install MySQL 8.0+
- Create a database (e.g., `deagentnet`)
- Note your connection credentials

## Configuration

### 1. Create Environment File

Copy the example configuration:
```bash
cp .env.example .env
```

### 2. Configure Required Variables

Edit `.env` and update these essential settings:

**Smart Contract Configuration:**
```env
# Use the deployed testnet contract or your own deployed contract
CONTRACT_PACKAGE_HASH=2a9d5cd5515245d2a50168c5d48e25e7dcc2b61bd7ca511e7b421ba623e45d19
```

**CSPR.cloud API Access:**
```env
# Get your access key from https://console.cspr.build
CSPR_CLOUD_ACCESS_KEY=your_access_key_here
```

**Database Connection:**
```env
# Default value for Docker setup
DB_URI="mysql://deagentnet:passw0rd@localhost:3306/deagentnet"
```

**Internal Service Authentication:**
```env
# Secret key for authorization headers between backend, server, and validators
INTERNAL_SERVICE_KEY=can_internal_secret_key_2026
```


## Installation

Install all dependencies:
```bash
npm install
```


## Running the Applications

### Development Mode

**Start the Event Listener:**
```bash
npm run event-handler:dev
```

This starts the listener with auto-reload on code changes. You should see:
```
[INFO] Handler started running...
[INFO] Connected to streaming API: wss://streaming.testnet.cspr.cloud
```

**Start the MCP Server:**
To start the MCP server in dev mode (Stdio):
```bash
npm run mcp:dev
```

For SSE mode (port 4000), set `MCP_SERVER_USE_SSE=true` and `PORT=4000` in `.env`, or use `ts-node src/mcp-server.ts` directly with SSE config.

## Troubleshooting

### Database Connection Issues

**Problem**: `Error: connect ECONNREFUSED 127.0.0.1:3306`

**Solution**:
- Ensure MySQL is running: `docker ps` or `systemctl status mysql`
- Verify credentials in `.env` match your database setup
- Check firewall isn't blocking port 3306

### CSPR.cloud Connection Issues

**Problem**: `Error: Unauthorized - Invalid access key`

**Solution**:
- Verify `CSPR_CLOUD_ACCESS_KEY` in `.env`
- Check key is active in [CSPR.build Console](https://console.cspr.build)
- Ensure no extra whitespace in the key value

### Event Listener Not Receiving Events

**Problem**: No events being processed

**Solution**:
- Verify `CONTRACT_PACKAGE_HASH` matches deployed contract
- Check contract has emitted events (view on [Testnet Explorer](https://testnet.cspr.live))
- Review logs for connection errors: `npm run event-handler:dev`

## Development Tips

### Watch Database Changes
```bash
# Connect to MySQL
docker exec -it agent-network-mysql mysql -u deagentnet -p

# Use database
USE deagentnet;

# View agents
SELECT * FROM agents ORDER BY timestamp DESC LIMIT 10;

# View tasks
SELECT * FROM tasks ORDER BY timestamp DESC LIMIT 10;
```

## Resources

- [Casper Network](https://casper.network) - Official website
- [CSPR.build Console](https://console.cspr.build) - Developer tools access
- [CSPR.cloud Documentation](https://docs.cspr.cloud/) - API reference
- [Testnet Explorer](https://testnet.cspr.live) - View transactions and contracts
- [Odra Framework](https://odra.dev/docs/) - Smart contract development

## Community & Support
Join [Casper Developers](https://t.me/CSPRDevelopers) Telegram channel to connect with other developers.
