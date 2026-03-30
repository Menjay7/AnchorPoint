import { Router, Request, Response } from 'express';
import { randomUUID } from 'crypto';
import {
  createDepositInteractiveUrl,
  createWithdrawInteractiveUrl,
  isSupportedAsset,
  normalizeAssetCode,
  SUPPORTED_ASSETS,
} from '../../services/kyc.service';

const router = Router();

interface InteractiveRequest {
  asset_code: string;
  account?: string;
  amount?: string;
  lang?: string;
}

interface InteractiveResponse {
  type: 'interactive_customer_info_needed';
  url: string;
  id: string;
}

const unsupportedAssetResponse = (assetCode: string) => ({
  error: `Asset ${assetCode} is not supported. Supported assets: ${SUPPORTED_ASSETS.join(', ')}`,
});

const getBaseInteractiveUrl = (): string => process.env.INTERACTIVE_URL || 'http://localhost:3000';

router.post('/transactions/deposit/interactive', (req: Request, res: Response) => {
  const { asset_code, account, amount, lang = 'en' }: InteractiveRequest = req.body;

  if (!asset_code) {
    return res.status(400).json({
      error: 'asset_code is required',
    });
  }

  const normalizedAssetCode = normalizeAssetCode(asset_code);
  if (!isSupportedAsset(normalizedAssetCode)) {
    return res.status(400).json(unsupportedAssetResponse(asset_code));
  }

  const transactionId = randomUUID();
  const response: InteractiveResponse = {
    type: 'interactive_customer_info_needed',
    url: createDepositInteractiveUrl({
      baseUrl: getBaseInteractiveUrl(),
      transactionId,
      assetCode: normalizedAssetCode,
      account,
      amount,
      lang,
    }),
    id: transactionId,
  };

  return res.json(response);
});

router.post('/transactions/withdraw/interactive', (req: Request, res: Response) => {
  const { asset_code, account, amount, lang = 'en' }: InteractiveRequest = req.body;

  if (!asset_code) {
    return res.status(400).json({
      error: 'asset_code is required',
    });
  }

  const normalizedAssetCode = normalizeAssetCode(asset_code);
  if (!isSupportedAsset(normalizedAssetCode)) {
    return res.status(400).json(unsupportedAssetResponse(asset_code));
  }

  const transactionId = randomUUID();
  const response: InteractiveResponse = {
    type: 'interactive_customer_info_needed',
    url: createWithdrawInteractiveUrl({
      baseUrl: getBaseInteractiveUrl(),
      transactionId,
      assetCode: normalizedAssetCode,
      account,
      amount,
      lang,
    }),
    id: transactionId,
  };

  return res.json(response);
});

export default router;
