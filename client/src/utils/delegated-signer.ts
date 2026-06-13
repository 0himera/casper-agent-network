import { PrivateKey, Transaction, PublicKey, KeyAlgorithm } from 'casper-js-sdk';

/**
 * Interface representing a Casper EIP-712 Typed Structured Message
 */
export interface CasperTypedMessage {
  domain: {
    name: string;
    version: string;
    chainId: string;
    verifyingContract?: string;
  };
  types: Record<string, Array<{ name: string; type: string }>>;
  primaryType: string;
  message: Record<string, any>;
}

/**
 * Signs an unsigned transaction JSON autonomously using a delegated private key.
 * Used for Mode B (Fully Autonomous Agent Transactions).
 * 
 * @param unsignedTxJson Transaction JSON payload returned from the MCP Server or indexer
 * @param privateKeyPem Private key in PEM format or raw hex string
 * @returns Signed transaction JSON payload ready for broadcast
 */
export const signTransactionAutonomously = (
  unsignedTxJson: any,
  privateKeyPem: string
): any => {
  // Try to load keypair from PEM string (Ed25519 first, fallback to Secp256k1)
  let key: PrivateKey;
  try {
    key = PrivateKey.fromPem(privateKeyPem, KeyAlgorithm.ED25519);
  } catch (e) {
    key = PrivateKey.fromPem(privateKeyPem, KeyAlgorithm.SECP256K1);
  }
  
  // Parse and sign the transaction object
  const txJson = unsignedTxJson.transaction || unsignedTxJson;
  const transaction = Transaction.fromJSON(txJson);
  transaction.sign(key);
  
  const signedJson = transaction.toJSON();
  if (unsignedTxJson.transaction) {
    return { transaction: signedJson };
  }
  return signedJson;
};

/**
 * Signs a Typed Structured Message (EIP-712 equivalent for Casper meta-transactions).
 * Allows agents to sign gasless actions which can be relayed/sponsored by the platform.
 * 
 * @param message Structured message conforming to the CasperTypedMessage specification
 * @param privateKeyPem Delegated private key
 * @returns Hex signature string (with algorithm tag prefix, 65 bytes / 130 hex characters)
 */
export const signTypedMessage = (
  message: CasperTypedMessage,
  privateKeyPem: string
): string => {
  let key: PrivateKey;
  try {
    key = PrivateKey.fromPem(privateKeyPem, KeyAlgorithm.ED25519);
  } catch (e) {
    key = PrivateKey.fromPem(privateKeyPem, KeyAlgorithm.SECP256K1);
  }
  
  // Serialize structured data for signing
  const serializedMessage = Buffer.from(JSON.stringify(message));
  const signature = key.signAndAddAlgorithmBytes(serializedMessage);
  
  return Buffer.from(signature).toString('hex');
};

/**
 * Verifies the signature of a CasperTypedMessage.
 * Used by the relayer (backend) to check client signatures before sponsoring gas.
 */
export const verifyTypedMessage = (
  message: CasperTypedMessage,
  signatureHex: string,
  publicKeyHex: string
): boolean => {
  const publicKey = PublicKey.fromHex(publicKeyHex);
  const serializedMessage = Buffer.from(JSON.stringify(message));
  let signatureBytes = Uint8Array.from(Buffer.from(signatureHex, 'hex'));
  
  // If the signature is raw (64 bytes), prepend the algorithm prefix tag (0x01 or 0x02)
  if (signatureBytes.length === 64) {
    const signatureWithTag = new Uint8Array(65);
    signatureWithTag[0] = publicKey.cryptoAlg; // 1 for Ed25519, 2 for Secp256k1
    signatureWithTag.set(signatureBytes, 1);
    signatureBytes = signatureWithTag;
  }
  
  return publicKey.verifySignature(serializedMessage, signatureBytes);
};


