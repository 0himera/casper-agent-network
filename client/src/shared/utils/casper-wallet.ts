import { RpcClient, HttpHandler, Transaction } from 'casper-js-sdk';

const NODE_URL = 'https://node.testnet.cspr.cloud/rpc'; // Casper Testnet RPC node
const rpcHandler = new HttpHandler(NODE_URL);
const casperClient = new RpcClient(rpcHandler);

export const getWalletProvider = () => {
  if (typeof window !== 'undefined' && (window as any).CasperWalletProvider) {
    try {
      return (window as any).CasperWalletProvider();
    } catch (e) {
      console.error('Failed to instantiate CasperWalletProvider', e);
    }
  }
  return null;
};

export const isWalletInstalled = (): boolean => {
  return typeof window !== 'undefined' && !!(window as any).CasperWalletProvider;
};

export const connectWallet = async (): Promise<string | null> => {
  const provider = getWalletProvider();
  if (!provider) return null;

  try {
    const connected = await provider.requestConnection();
    if (connected) {
      return await provider.getActivePublicKey();
    }
  } catch (err) {
    console.error('Error connecting to Casper Wallet', err);
  }
  return null;
};

export const disconnectWallet = async (): Promise<boolean> => {
  const provider = getWalletProvider();
  if (!provider) return false;

  try {
    return await provider.disconnectFromSite();
  } catch (err) {
    console.error('Error disconnecting Casper Wallet', err);
    return false;
  }
};

export const getConnectedAccount = async (): Promise<string | null> => {
  const provider = getWalletProvider();
  if (!provider) return null;

  try {
    const isConnected = await provider.isConnected();
    if (isConnected) {
      return await provider.getActivePublicKey();
    }
  } catch (err) {
    console.error('Error checking connection status', err);
  }
  return null;
};

/**
 * Signs and broadcasts a Casper transaction using the browser Casper Wallet extension.
 *
 * @param transaction The Transaction object constructed from casper-js-sdk
 * @param senderHex The public key hex of the sender
 * @returns The deploy/transaction hash of the broadcasted transaction
 */
export const signAndSendTransaction = async (
  transaction: any,
  senderHex: string
): Promise<string> => {
  const provider = getWalletProvider();
  if (!provider) {
    throw new Error('Casper Wallet is not installed or available.');
  }

  // Convert transaction to JSON format required by the wallet extension
  const transactionJson = transaction.toJSON();
  const signResult = await provider.sign(
    JSON.stringify({ transaction: transactionJson }),
    senderHex
  );

  if (signResult.cancelled) {
    throw new Error('Transaction signing was cancelled by user.');
  }

  // Parse signed transaction JSON from wallet response
  const signedTxJson = signResult.transaction || signResult;
  const signedTransaction = Transaction.fromJSON(signedTxJson);

  // Broadcast the transaction to the Casper node RPC
  const result = await casperClient.putTransaction(signedTransaction);
  if (result.transactionHash) {
    return typeof result.transactionHash.toHex === 'function'
      ? result.transactionHash.toHex()
      : String(result.transactionHash);
  }
  return String(result);
};
