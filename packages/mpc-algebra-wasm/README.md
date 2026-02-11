# mpc-algebra-wasm

This library provides Zero-Knowledge Proof (ZKP) circuits specifically designed for the Werewolf game implementation. It's built to be compiled to WebAssembly (wasm) and used in JavaScript/TypeScript environments.

## Overview

The library includes five essential ZKP circuits for the Werewolf game:
1. Anonymous Voting Circuit
2. Role Assignment Circuit
3. Winning Judgement Circuit
4. Key Publicize Circuit
5. Divination Circuit

## Usage

The library is designed to be used as a WebAssembly module in JavaScript/TypeScript applications. Each circuit takes both private and public inputs, encrypts the private inputs, and returns the encrypted data in a format ready to be sent via HTTP requests.

### Data Flow
1. Circuit receives private and public inputs
2. Private inputs are encrypted using the specified encryption scheme
3. Returns encrypted data in a format suitable for HTTP transmission
4. The encrypted output can be directly used in API requests

## Building

Build as WebAssembly:
```bash
wasm-pack build --target web --out-dir pkg-web
```