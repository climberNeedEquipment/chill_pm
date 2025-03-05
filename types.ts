/**
 * TypeScript definitions translated from Rust types
 */

// Equivalent to the Message struct
export interface Message {
  role: string;
  content: string;
}

// Equivalent to the Exchanges struct
export interface Exchanges {
  binance: BinanceExchange;
  eisen: EisenExchange;
}

// Equivalent to the BinanceExchange struct
export interface BinanceExchange {
  orders?: BinanceOrder[];
}

// Equivalent to the BinanceOrder struct
export interface BinanceOrder {
  position: string;
  token: string;
  amount: string;
  price: string;
  side: string;
}

// Equivalent to the EisenExchange struct
export interface EisenExchange {
  swaps?: EisenSwap[];
}

// Equivalent to the EisenSwap struct
// Note: Using camelCase as specified in the Rust serde attribute
export interface EisenSwap {
  tokenIn: string;
  tokenOut: string;
  amount: string;
}

// Equivalent to the Strategy struct
export interface Strategy {
  exchanges: Exchanges;
}

// Equivalent to the Agent trait (as an interface in TypeScript)
export interface Agent {
  setPrompt(prompt: string): Agent;
  chat(messages: Message[]): Promise<string>;
  prompt(): string;
} 