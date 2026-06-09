export * from './theme';
export * from './contract-transactions';

export const formatAddress = (address?: string) => {
  if (!address) return '';
  return `${address.substring(0, 6)}...${address.substring(address.length - 4)}`;
};
