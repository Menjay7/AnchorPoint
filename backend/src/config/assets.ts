/**
 * Multi-asset configuration for the anchor.
 * Add or remove assets here to dynamically update all SEP endpoints.
 */

export interface AssetConfig {
  code: string;
  issuer?: string; // Stellar issuer public key (undefined for native XLM)
  type: 'fiat' | 'crypto' | 'other';
  desc: string;
  minAmount: string;
  maxAmount: string;
  feeFixed: number;
  feePercent: number;
  feeMinimum: number;
  depositEnabled: boolean;
  withdrawEnabled: boolean;
}

export const ASSETS: AssetConfig[] = [
  {
    code: 'USDC',
    issuer: process.env.USDC_ISSUER || 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    type: 'fiat',
    desc: 'USD Coin - a fully collateralized US dollar stablecoin',
    minAmount: '0.01',
    maxAmount: '1000000',
    feeFixed: 0.5,
    feePercent: 0.005,
    feeMinimum: 0.5,
    depositEnabled: true,
    withdrawEnabled: true,
  },
  {
    code: 'USD',
    type: 'fiat',
    desc: 'US Dollar - traditional currency',
    minAmount: '0.01',
    maxAmount: '1000000',
    feeFixed: 0.5,
    feePercent: 0.005,
    feeMinimum: 0.5,
    depositEnabled: true,
    withdrawEnabled: true,
  },
  {
    code: 'BTC',
    type: 'crypto',
    desc: 'Bitcoin - decentralized digital currency',
    minAmount: '0.00001',
    maxAmount: '100',
    feeFixed: 0.001,
    feePercent: 0.01,
    feeMinimum: 0.001,
    depositEnabled: true,
    withdrawEnabled: true,
  },
  {
    code: 'ETH',
    type: 'crypto',
    desc: 'Ethereum - smart contract platform',
    minAmount: '0.001',
    maxAmount: '1000',
    feeFixed: 0.01,
    feePercent: 0.01,
    feeMinimum: 0.01,
    depositEnabled: true,
    withdrawEnabled: true,
  },
];

/** Map of asset code -> AssetConfig for O(1) lookups */
export const ASSET_MAP: Record<string, AssetConfig> = Object.fromEntries(
  ASSETS.map(a => [a.code, a])
);

export const SUPPORTED_ASSET_CODES = ASSETS.map(a => a.code);

export const getAsset = (code: string): AssetConfig | undefined =>
  ASSET_MAP[code.trim().toUpperCase()];

export const isDepositSupported = (code: string): boolean =>
  getAsset(code)?.depositEnabled ?? false;

export const isWithdrawSupported = (code: string): boolean =>
  getAsset(code)?.withdrawEnabled ?? false;
