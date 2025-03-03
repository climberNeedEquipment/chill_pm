use crate::agent::Agent;
use crate::agent::Message;
use anyhow::Result;

pub struct StableYieldFarmingAgent<A: Agent> {
  inner: A,
}

impl<A: Agent> StableYieldFarmingAgent<A> {
  pub fn new(mut agent: A) -> Self {
      // Set the specialized finance prompt
      agent.set_prompt(String::from(
          "You are a specialized financial advisor focused on stable yield farming strategies. \
          Provide conservative, well-researched advice on DeFi protocols, yield optimization, \
          risk assessment, and portfolio diversification. Always prioritize security and \
          sustainability over high APYs. Include relevant warnings about smart contract risks, \
          impermanent loss, and market volatility where appropriate.",
      ));

      Self { inner: agent }
  }

  // Delegate the chat method to the inner Agent
  pub async fn chat(&self, messages: Vec<Message>) -> Result<String> {
      // Create a new vector with the system prompt as the first message
      let mut all_messages = vec![Message {
          role: "system".to_string(),
          content: self.inner.prompt().to_string(),
      }];

      // Add the user messages
      all_messages.extend(messages);

      // Call the inner agent's chat method
      self.inner.chat(all_messages).await
  }

  pub async fn get_farming_strategy(
      &self,
      prices: &String,
      portfolio_summary: &String,
  ) -> Result<String> {
      // Fetch the user's portfolio data for the specific token

      // Create a message asking for yield farming advice based on the portfolio
      let messages = vec![
          Message {
              role: "user".to_string(),
              content: format!(
                  "I have the following portfolio:\n\n{}\n\n
                  Here is the current market price of the tokens in the portfolio:\n\n{}\n\n
                  I want to optimize my yield farming \
                  strategy. \n\n\
                  Please recommend a strategy that is delta neutral, meaning you should take both opposite positions between CEX and DEX. \
                  The Eisen portfoilio is for DEX, and Binance is for CEX.
                  In Binance, you can only trade on BTC and ETH
                  In Eisen, you can trade on all the tokens in the portfolio.
                  Here is an example of ouput format that should be in JSON format do not print anything else:\
                  {}",
                  portfolio_summary,
                  prices,
                  r#"
{
  "exchanges": [
      {
          "target": "Binance",
          "positions": [
              {
                  "position": "short",
                  "token": "<token_symbol1>",
                  "amount": "<amount>",
                  "price": "<price>",
                  "side": "sell"
              },
              {
                  "position": "short",
                  "token": "<token_symbol2>",
                  "amount": "<amount>",
                  "price": "<price>",
                  "side": "sell"
              }
          ]   
      },
      {
          "target": "Eisen",
          "swaps": [
              {
                  "token_in": "mETH",
                  "token_out": "ETH",
                  "amount": "<amount>",
              },
              {
                  "token_in": "stETH",
                  "token_out": "ETH",
                  "amount": "<amount>",
              }
          ]
      }
  ]
}
                  "#
              ),
          },
      ];

      // Get the AI's recommendation
      self.chat(messages).await
  }
}
