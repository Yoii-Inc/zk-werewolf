FROM ghcr.io/foundry-rs/foundry:latest

# Set the working directory
WORKDIR /app

# Copy the foundry project
COPY . .

# Expose the port for Anvil
EXPOSE 8545

# Start Anvil (local Ethereum node)
ENTRYPOINT ["anvil", "--host", "0.0.0.0", "--port", "8545"]