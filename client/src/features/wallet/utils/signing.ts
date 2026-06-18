import type { SendResult } from "@make-software/csprclick-core-types";
import { walletStore } from "../store/wallet-store";

/**
 * Sign and broadcast a transaction via CSPR.click.
 * Replaces the old casper-wallet.ts `signAndSendTransaction`.
 */
export async function signAndSendTransaction(
  transaction: any,
  senderHex: string,
): Promise<string> {
  const storeProvider = walletStore.getState().provider;
  if (storeProvider === "mock") {
    console.warn("Dev mode: bypassing transaction signing. Generating mock transaction hash.");
    const randHex = Array.from({ length: 32 }, () =>
      Math.floor(Math.random() * 16).toString(16)
    ).join("");
    return `mock-tx-${randHex}`;
  }

  const csprclick = window.csprclick;
  if (!csprclick) throw new Error("CSPR.click not initialized");

  const txJson = typeof transaction.toJSON === "function"
    ? (transaction.toJSON() as object)
    : transaction;

  const result = await csprclick.send(txJson, senderHex.toLowerCase());

  if (!result) throw new Error("No result from CSPR.click send");

  const r = result as SendResult;
  if (r.cancelled) throw new Error("Transaction signing was cancelled by user");
  if (r.error) throw new Error(`Transaction failed: ${r.error}`);
  if (!r.transactionHash && !r.deployHash) throw new Error("No transaction hash returned");

  return (r.transactionHash || r.deployHash) as string;
}
