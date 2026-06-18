import type { SendResult } from "@make-software/csprclick-core-types";

/**
 * Sign and broadcast a transaction via CSPR.click.
 * Replaces the old casper-wallet.ts `signAndSendTransaction`.
 */
export async function signAndSendTransaction(
  transaction: any,
  senderHex: string,
): Promise<string> {
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
