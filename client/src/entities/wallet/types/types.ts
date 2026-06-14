export type CasperNetwork = "mainnet" | "testnet";

export interface WalletState {
  address: string | null;
  balance: number;
  isConnected: boolean;
  network: CasperNetwork;
}

export type TransactionStep = "preparing" | "signing" | "broadcasted" | "processed" | "failed";

export interface TransactionState {
  step: TransactionStep;
  hash: string | null;
  blockNumber: number | null;
  gasUsed: number | null;
  errorMessage: string | null;
}
