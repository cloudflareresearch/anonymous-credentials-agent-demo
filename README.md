# 🦊 ACT Rate-Limiting Demo

![GitHub License](https://img.shields.io/github/license/cloudflareresearch/anonymous-credentials-agent-demo)

Interaction between client and issuer to obtain and spend anonymous credit tokens (ACT).

- **Issuer** <https://act-issuer-demo.cloudflareresearch.com/>
- **Client** <https://act-client-demo.cloudflareresearch.com/>

Read the blog post: <https://blog.cloudflare.com/private-rate-limiting>

## Content

- [Usage](#usage)
  - [Issuer](#issuer)
  - [Client](#client)
  - [MCP Server](#mcp-server)
- [Security Considerations](#security-considerations)
- [License](#license)

## Usage

### Issuer

```sh
cd issuer
npm i
npx wrangler deploy
```

### Client

```sh
cd client
npm i
npm run build
npm run deploy
```

### MCP Server

The issuer needs to be started for the mcp server to work properly.
The following command starts the MCP inspector to manually trigger tool calls.

```sh
cd act-mcp-server
cargo build
npx @modelcontextprotocol/inspector cargo run --bin act-mcp-server
```

## Security Considerations

This software has not been audited. Please use at your sole discretion.

## License

This project is under the Apache 2.0 license.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be Apache 2.0 licensed as above, without any additional terms or conditions.
