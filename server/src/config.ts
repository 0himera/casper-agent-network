import * as process from 'process';
import dotenv from 'dotenv';

dotenv.config();

interface Config {
  httpPort: number;
  csprCloudApiUrl: string;
  csprCloudStreamingUrl: string;
  csprCloudAccessKey: string;
  contractPackageHash: string;
  nodeUrl: string;
  dbURI: string;
  pingCheckIntervalInMilliseconds: number;
  useSmoothedLeaderboard: boolean;
}

export const config: Config = {
  httpPort: process.env.HTTP_PORT ? parseInt(process.env.HTTP_PORT) : 4000,
  csprCloudApiUrl: process.env.CSPR_CLOUD_URL as string,
  csprCloudStreamingUrl: process.env.CSPR_CLOUD_STREAMING_URL as string,
  csprCloudAccessKey: process.env.CSPR_CLOUD_ACCESS_KEY as string,
  contractPackageHash: process.env.CONTRACT_PACKAGE_HASH as string,
  nodeUrl: process.env.CASPER_NODE_URL || 'https://node.testnet.casper.network/rpc',
  dbURI: process.env.DB_URI as string,
  pingCheckIntervalInMilliseconds: 60000,
  useSmoothedLeaderboard: process.env.EXAM_LEADERBOARD_USE_SMOOTHED === '1' || process.env.EXAM_LEADERBOARD_USE_SMOOTHED?.toLowerCase() === 'true',
};
