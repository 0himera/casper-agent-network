import {
  Args,
  CLTypeUInt8,
  CLValue,
  Hash,
  PublicKey,
  SessionBuilder,
  Key,
  NativeTransferBuilder
} from 'casper-js-sdk';

const getProxyWasm = async (): Promise<Uint8Array> => {
  const res = await fetch('/proxy_caller.wasm');
  if (!res.ok) {
    throw new Error('Failed to fetch proxy WASM from /proxy_caller.wasm');
  }
  const buffer = await res.arrayBuffer();
  return new Uint8Array(buffer);
};

export const buildContractTransaction = async (
  senderHex: string,
  entryPoint: string,
  innerArgsMap: Record<string, CLValue>,
  attachedMotes: string = '0'
): Promise<any> => {
  const contractWasm = await getProxyWasm();
  const packageHash = process.env.NEXT_PUBLIC_CONTRACT_PACKAGE_HASH || 'e8e0cba1a3e6c8d2f17a51066d60ebaae764e54e5476ebb965eadff6e56dc699';

  const innerArgs = Args.fromMap(innerArgsMap);

  const serializedArgs = CLValue.newCLList(
    CLTypeUInt8,
    Array.from(innerArgs.toBytes()).map((value) => CLValue.newCLUint8(value))
  );

  const args = Args.fromMap({
    amount: CLValue.newCLUInt512(attachedMotes),
    attached_value: CLValue.newCLUInt512(attachedMotes),
    entry_point: CLValue.newCLString(entryPoint),
    package_hash: CLValue.newCLByteArray(Hash.fromHex(packageHash).toBytes()),
    args: serializedArgs
  });

  const payment = 10_000_000_000; // 10 CSPR payment for contract interaction

  const sessionTransaction = new SessionBuilder()
    .from(PublicKey.fromHex(senderHex))
    .runtimeArgs(args)
    .wasm(contractWasm)
    .payment(payment)
    .chainName('casper-test')
    .build();

  return sessionTransaction;
};

// High-level transaction builders
export const buildRegisterAgentTx = async (
  senderHex: string,
  name: string,
  description: string,
  metadataUri: string
) => {
  return buildContractTransaction(senderHex, 'register_agent', {
    name: CLValue.newCLString(name),
    description: CLValue.newCLString(description),
    metadata_uri: CLValue.newCLString(metadataUri)
  });
};

export const buildCreateTaskTx = async (
  senderHex: string,
  taskId: string,
  budgetMotes: string,
  metadataUri: string,
  deadline: number
) => {
  return buildContractTransaction(senderHex, 'create_task', {
    task_id: CLValue.newCLString(taskId),
    metadata_uri: CLValue.newCLString(metadataUri),
    deadline: CLValue.newCLUint64(deadline)
  }, budgetMotes);
};

export const buildAssignTaskTx = async (
  senderHex: string,
  taskId: string,
  agentHex: string
) => {
  const agentKeyStr = PublicKey.fromHex(agentHex).accountHash().toPrefixedString();
  const agentKey = Key.newKey(agentKeyStr);

  return buildContractTransaction(senderHex, 'assign_task', {
    task_id: CLValue.newCLString(taskId),
    agent: CLValue.newCLKey(agentKey)
  });
};

export const buildSetPriceTx = async (
  senderHex: string,
  priceMotes: string
) => {
  return buildContractTransaction(senderHex, 'set_price', {
    price: CLValue.newCLUInt512(priceMotes)
  });
};

export const buildCancelTaskTx = async (
  senderHex: string,
  taskId: string
) => {
  return buildContractTransaction(senderHex, 'cancel_task', {
    task_id: CLValue.newCLString(taskId)
  });
};

export const buildNativeTransferTx = (
  senderHex: string,
  recipientHex: string,
  amountMotes: string,
  transferId: number = Date.now()
) => {
  return new NativeTransferBuilder()
    .from(PublicKey.fromHex(senderHex))
    .target(PublicKey.fromHex(recipientHex))
    .amount(amountMotes)
    .id(transferId)
    .chainName('casper-test')
    .payment(100_000_000) // 0.1 CSPR gas fee
    .build();
};
