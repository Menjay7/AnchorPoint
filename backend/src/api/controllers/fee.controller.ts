import { Request, Response } from 'express';
import { FeeService } from '../../services/fee.service';

/**
 * GET /fees/stats
 * Returns raw network fee statistics from Horizon.
 */
export async function getFeeStats(req: Request, res: Response, feeService: FeeService): Promise<void> {
  try {
    const stats = await feeService.getFeeStats();
    res.json(stats);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Failed to fetch fee stats';
    res.status(502).json({ error: message });
  }
}

/**
 * GET /fees/estimate?operations=1
 * Returns an estimated transaction fee for the given number of operations.
 */
export async function estimateFee(req: Request, res: Response, feeService: FeeService): Promise<void> {
  const raw = parseInt(req.query.operations as string, 10);
  const operationCount = Number.isFinite(raw) && raw > 0 ? raw : 1;

  try {
    const estimate = await feeService.estimateFee(operationCount);
    res.json(estimate);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Failed to estimate fee';
    res.status(502).json({ error: message });
  }
}
