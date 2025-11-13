# Stage 1: Build WASM
FROM rust:1.83 AS wasm-builder

# Install wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /build

# Copy entire workspace
COPY . .

# Build WASM package
WORKDIR /build/packages/mpc-algebra-wasm
RUN wasm-pack build --target nodejs --out-dir pkg-node

# Stage 2: Build Next.js app
FROM node:18

WORKDIR /workspace

# Copy workspace files
COPY package.json package-lock.json* ./
COPY packages ./packages

# Copy built WASM from previous stage
COPY --from=wasm-builder /build/packages/mpc-algebra-wasm/pkg-node ./packages/mpc-algebra-wasm/pkg-node

# Install dependencies
RUN npm install

# Set working directory to nextjs package
WORKDIR /workspace/packages/nextjs

# Build arguments for NEXT_PUBLIC_* variables
ARG NEXT_PUBLIC_API_URL
ARG NEXT_PUBLIC_WS_URL

# Convert to environment variables for build
ENV NEXT_PUBLIC_API_URL=$NEXT_PUBLIC_API_URL
ENV NEXT_PUBLIC_WS_URL=$NEXT_PUBLIC_WS_URL

RUN npm run build

EXPOSE 3000

ENTRYPOINT ["npm", "run", "serve"]
