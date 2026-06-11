import {
  Args,
  CLTypeUInt8,
  CLValue,
  Hash,
  PublicKey,
  SessionBuilder,
  Key
} from 'casper-js-sdk';

const getProxyWasm = async (): Promise<Uint8Array> => {
  const configApiUrl = (window as any).config?.agent_network_api_url || 'http://localhost:4000';
  const res = await fetch(`${configApiUrl}/proxy-wasm`);
  if (!res.ok) {
    throw new Error(`Failed to fetch proxy WASM from ${configApiUrl}/proxy-wasm`);
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
  const packageHash = (window as any).config?.agent_network_contract_package_hash;
  if (!packageHash) {
    throw new Error("Missing agent_network_contract_package_hash in client config");
  }

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

  const payment = Number.parseInt((window as any).config?.transaction_payment || '10000000000', 10);

  const sessionTransaction = new SessionBuilder()
    .from(PublicKey.fromHex(senderHex))
    .runtimeArgs(args)
    .wasm(contractWasm)
    .payment(payment)
    .chainName(window.csprclick?.chainName || 'casper-test')
    .build();

  return {
    transaction: {
      Version1: sessionTransaction.toJSON()
    }
  };
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

export const buildSubmitResultTx = async (
  senderHex: string,
  taskId: string,
  resultHash: string
) => {
  return buildContractTransaction(senderHex, 'submit_result', {
    task_id: CLValue.newCLString(taskId),
    result_hash: CLValue.newCLString(resultHash)
  });
};

export const buildCompleteTaskTx = async (
  senderHex: string,
  taskId: string,
  skill: string,
  score: number,
  weight: number
) => {
  return buildContractTransaction(senderHex, 'complete_task', {
    task_id: CLValue.newCLString(taskId),
    skill: CLValue.newCLString(skill),
    score: CLValue.newCLUInt32(score),
    weight: CLValue.newCLUInt32(weight)
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

export const buildSetPriceTx = async (
  senderHex: string,
  priceMotes: string
) => {
  return buildContractTransaction(senderHex, 'set_price', {
    price: CLValue.newCLUInt512(priceMotes)
  });
};

export const buildUpdateRecommendedPriceTx = async (
  senderHex: string,
  agentHex: string,
  priceMotes: string
) => {
  const agentKeyStr = PublicKey.fromHex(agentHex).accountHash().toPrefixedString();
  const agentKey = Key.newKey(agentKeyStr);

  return buildContractTransaction(senderHex, 'update_recommended_price', {
    agent: CLValue.newCLKey(agentKey),
    price: CLValue.newCLUInt512(priceMotes)
  });
};
