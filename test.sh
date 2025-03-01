#!/bin/bash

# Define variables
HOST="localhost"
PORT="8080"
ENDPOINT="/api/v1/execute"
URL="http://${HOST}:${PORT}${ENDPOINT}"
WALLET_ADDRESS="0x96a2e1Cb03128DC4cD2b5D9502F0AaB8E9a9e856"  # Example Ethereum wallet address

# Print information about the request
echo "Testing execute endpoint at: ${URL}"
echo "Using wallet address: ${WALLET_ADDRESS}"
echo "Sending request..."

# Send the POST request with curl
curl -X POST "${URL}" \
  -H "Content-Type: application/json" \
  -d "{\"wallet_address\":\"${WALLET_ADDRESS}\"}" \
  -v

echo -e "\n\nRequest completed."
