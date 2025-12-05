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

# Stage 2: Dependencies
FROM node:18 AS deps

WORKDIR /app

# Copy package.json files
COPY package.json yarn.lock .yarnrc.yml ./
COPY .yarn ./.yarn
COPY packages/nextjs/package.json ./packages/nextjs/

# Install dependencies
RUN yarn install && yarn cache clean

# Stage 3: Builder
FROM node:18 AS builder

WORKDIR /app
# Copy workspace configuration files
COPY package.json yarn.lock .yarnrc.yml ./
COPY .yarn ./.yarn

# Copy dependencies from deps stage
COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/packages/nextjs/node_modules ./packages/nextjs/node_modules

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

# Stage 4: Runner - Production runtime
FROM node:18 AS runner

WORKDIR /app

# copy workspace files
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/yarn.lock ./yarn.lock
COPY --from=builder /app/.yarnrc.yml ./.yarnrc.yml
COPY --from=builder /app/.yarn ./.yarn

# Copy built application from builder stage
COPY --from=builder /app/packages/nextjs/.next ./packages/nextjs/.next
COPY --from=builder /app/packages/nextjs/public ./packages/nextjs/public
COPY --from=builder /app/packages/nextjs/package.json ./packages/nextjs/package.json

COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/packages/nextjs/node_modules ./packages/nextjs/node_modules

EXPOSE 3000

ENV NODE_ENV=production

CMD ["yarn", "start"]
