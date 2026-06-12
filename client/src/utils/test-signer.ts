import { PrivateKey, KeyAlgorithm, PublicKey, SessionBuilder, Args } from 'casper-js-sdk';
import { signTypedMessage, verifyTypedMessage, signTransactionAutonomously } from './delegated-signer';

async function testSigner() {
  console.log('--- Casper Delegated Signer Test ---');
  
  // 1. Generate an Ed25519 keypair
  const privateKey = PrivateKey.generate(KeyAlgorithm.ED25519);
  const publicKey = privateKey.publicKey;
  const privateKeyPem = privateKey.toPem();
  const publicKeyHex = publicKey.toHex();
  
  console.log('Generated Public Key:', publicKeyHex);
  
  // 2. Test Typed Message signing and verification
  const typedMessage = {
    domain: {
      name: 'Casper Agent Network',
      version: '1.0.0',
      chainId: 'casper-test',
    },
    types: {
      AgentAction: [
        { name: 'agent', type: 'string' },
        { name: 'action', type: 'string' },
        { name: 'nonce', type: 'uint64' },
      ],
    },
    primaryType: 'AgentAction',
    message: {
      agent: publicKeyHex,
      action: 'query_reputation',
      nonce: 42,
    },
  };
  
  const signatureHex = signTypedMessage(typedMessage, privateKeyPem);
  console.log('Signed Typed Message signature (hex):', signatureHex);
  console.log('Signed Typed Message signature (length):', signatureHex.length);
  
  const signatureBytes = Uint8Array.from(Buffer.from(signatureHex, 'hex'));
  console.log('Raw signature bytes length:', signatureBytes.length);
  
  // Try directly using verifyTypedMessage
  let isValid = false;
  try {
    isValid = verifyTypedMessage(typedMessage, signatureHex, publicKeyHex);
    console.log('verifyTypedMessage result:', isValid);
  } catch (err: any) {
    console.log('verifyTypedMessage threw error:', err.message);
  }

  // Experiment directly with PublicKey.verifySignature
  const pk = PublicKey.fromHex(publicKeyHex);
  const serialized = Buffer.from(JSON.stringify(typedMessage));
  
  // 1. Try with raw 64 bytes
  try {
    const res64 = pk.verifySignature(serialized, signatureBytes);
    console.log('Direct verify with raw 64 bytes:', res64);
  } catch (err: any) {
    console.log('Direct verify with raw 64 bytes threw:', err.message);
  }

  // 2. Try prepending Ed25519 tag (0x01) to make 65 bytes
  try {
    const signatureWithTag = new Uint8Array(65);
    signatureWithTag[0] = 1; // Ed25519
    signatureWithTag.set(signatureBytes, 1);
    const res65 = pk.verifySignature(serialized, signatureWithTag);
    console.log('Direct verify with 65 bytes (with tag):', res65);
  } catch (err: any) {
    console.log('Direct verify with 65 bytes threw:', err.message);
  }

  
  if (!isValid) {
    throw new Error('Signature verification failed!');
  }
  console.log('✅ Typed message signature successfully verified!');
  
  // 3. Test Transaction Autonomous Signing
  // Construct a real transaction using SessionBuilder
  const sessionTransaction = new SessionBuilder()
    .from(publicKey)
    .chainName('casper-test')
    .payment(10000000000)
    .wasm(new Uint8Array([1, 2, 3, 4])) // dummy wasm bytes
    .runtimeArgs(Args.fromMap({}))
    .build();
    
  const unsignedTxJson = sessionTransaction.toJSON();
  
  try {
    const signedTxJson = signTransactionAutonomously(unsignedTxJson, privateKeyPem);
    
    // In casper-js-sdk v5, transaction approvals is an array under signedTxJson
    console.log('Signed Tx Approvals length:', signedTxJson.approvals?.length);
    if (signedTxJson.approvals?.length > 0) {
      console.log('✅ Transaction successfully signed autonomously!');
    } else {
      throw new Error('Transaction was not signed!');
    }
  } catch (err) {
    console.error('Transaction signing failed:', err);
  }
}

testSigner().catch(console.error);

