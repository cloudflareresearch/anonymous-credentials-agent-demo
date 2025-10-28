# ACT Demo - Issuer

This is a Cloudflare Worker written in rust for issuing ACT tokens.

## Deploying the code

```sh
npm ci
npm run deploy
```

## Key Generation

This is one-time setup step for generating the issuer's private key.

```sh
npm run keygen
npm run upload-keygen
```
