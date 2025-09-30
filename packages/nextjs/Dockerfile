FROM node:18

WORKDIR /app

# Copy package files
COPY package.json package-lock.json* /workspace/

# Set workspace structure
RUN mkdir -p /workspace/packages/nextjs
COPY packages/nextjs/package.json /workspace/packages/nextjs/package.json
COPY packages/nextjs /workspace/packages/nextjs

# Install dependencies from workspace root
WORKDIR /workspace
RUN npm install

# Set working directory back to nextjs
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