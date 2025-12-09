# Stage 1: Build WASM
FROM rust:1.83 AS wasm-builder

# Install wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /build

# Create a minimal workspace for WASM build only
RUN mkdir -p packages/mpc-algebra-wasm

# Copy mpc-algebra-wasm package
COPY packages/mpc-algebra-wasm ./packages/mpc-algebra-wasm

# Build WASM package directly without workspace
WORKDIR /build/packages/mpc-algebra-wasm
RUN wasm-pack build --target web --out-dir pkg

# Stage 2: Dependencies & Builder
FROM node:18-alpine AS deps

# Install build dependencies for native modules
RUN apk add --no-cache python3 make g++

WORKDIR /app

# Copy package.json files
COPY package.json yarn.lock .yarnrc.yml ./
COPY .yarn ./.yarn
COPY packages/nextjs/package.json ./packages/nextjs/

# Install dependencies
RUN yarn install

# Copy source code to current directory
COPY packages/nextjs ./packages/nextjs
COPY --from=wasm-builder /build/packages/mpc-algebra-wasm/pkg ./packages/mpc-algebra-wasm/pkg-node

# Build arguments
ARG NEXT_PUBLIC_API_URL
ARG NEXT_PUBLIC_WS_URL

# Convert to environment variables for build
ENV NEXT_PUBLIC_API_URL=$NEXT_PUBLIC_API_URL
ENV NEXT_PUBLIC_WS_URL=$NEXT_PUBLIC_WS_URL
ENV NODE_ENV=production

RUN yarn workspace @se-2/nextjs build
RUN yarn workspaces focus @se-2/nextjs --production

# Stage 3: Runner - Production runtime (standalone)
FROM node:18-alpine AS runner

WORKDIR /app

# Copy standalone build (includes minimal node server)
COPY --from=deps /app/packages/nextjs/.next/standalone ./

# Copy static files
COPY --from=deps /app/packages/nextjs/.next/static ./packages/nextjs/.next/static
COPY --from=deps /app/packages/nextjs/public ./packages/nextjs/public

EXPOSE 3000

ENV NODE_ENV=production
ENV HOSTNAME="0.0.0.0"

CMD ["node", "packages/nextjs/server.js"]
