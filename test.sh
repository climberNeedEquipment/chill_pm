#!/bin/bash

# Define variables
HOST="localhost"
PORT="8080"
ENDPOINT="/api/v1/execute"
URL="http://${HOST}:${PORT}${ENDPOINT}"
WALLET_ADDRESS="0x998F8207Ab6Ea84d0124232529de02d537102c85"  # Example Ethereum wallet address

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
