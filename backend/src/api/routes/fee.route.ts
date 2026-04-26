import { Router, Request, Response } from 'express';
import { FeeService } from '../../services/fee.service';
import { RedisService } from '../../services/redis.service';
import { getFeeStats, estimateFee } from '../controllers/fee.controller';

const router = Router();

// Mock Redis for environments without a real Redis instance
const mockRedisClient = {
  get: async () => null,
  set: async () => {},
  del: async () => 1,
  expire: async () => {},
};

const redisService = new RedisService(mockRedisClient);
const feeService = new FeeService(redisService);

/**
 * @swagger
 * /fees/stats:
 *   get:
 *     summary: Network fee statistics
 *     description: Returns current Stellar network fee stats including surge status and percentile fees.
 *     tags: [Fees]
 *     responses:
 *       200:
 *         description: Fee statistics
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 baseFeeStroops:
 *                   type: number
 *                   example: 100
 *                 surgeActive:
 *                   type: boolean
 *                   example: false
 *                 surgeMultiplier:
 *                   type: number
 *                   example: 1.0
 *                 recommendedFeeStroops:
 *                   type: number
 *                   example: 200
 *                 p10FeeStroops:
 *                   type: number
 *                 p50FeeStroops:
 *                   type: number
 *                 p95FeeStroops:
 *                   type: number
 *                 ledgerCapacityUsage:
 *                   type: number
 *                   example: 0.45
 *                 fetchedAt:
 *                   type: string
 *                   format: date-time
 *       502:
 *         description: Failed to reach Horizon
 */
router.get('/stats', (req: Request, res: Response) => {
  return getFeeStats(req, res, feeService);
});

/**
 * @swagger
 * /fees/estimate:
 *   get:
 *     summary: Estimate transaction fee
 *     description: Returns an estimated fee for a transaction with the given number of operations.
 *     tags: [Fees]
 *     parameters:
 *       - in: query
 *         name: operations
 *         schema:
 *           type: integer
 *           default: 1
 *         description: Number of operations in the transaction
 *     responses:
 *       200:
 *         description: Fee estimate
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 estimatedFeeStroops:
 *                   type: number
 *                   example: 200
 *                 estimatedFeeXLM:
 *                   type: string
 *                   example: "0.0000200"
 *                 surgeActive:
 *                   type: boolean
 *                 surgeMultiplier:
 *                   type: number
 *                 operationCount:
 *                   type: number
 *       502:
 *         description: Failed to reach Horizon
 */
router.get('/estimate', (req: Request, res: Response) => {
  return estimateFee(req, res, feeService);
});

export default router;
