import { Router, Request, Response } from 'express';
import { RedisService } from '../../services/redis.service';
import { getChallenge, getToken } from '../controllers/auth.controller';

const router = Router();

// Mock Redis client for demonstration
// In a real implementation, you would inject the actual Redis client
const mockRedisClient = {
  get: async (key: string) => null,
  set: async (key: string, value: string) => {},
  del: async (key: string) => 1,
  expire: async (key: string, seconds: number) => {}
};

const redisService = new RedisService(mockRedisClient);

/**
 * @swagger
 * /auth:
 *   post:
 *     summary: SEP-10 Challenge Endpoint
 *     description: Generates a SEP-10 challenge transaction for client authentication
 *     tags: [Auth]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required:
 *               - account
 *             properties:
 *               account:
 *                 type: string
 *                 description: Stellar account public key
 *                 example: GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
 *               home_domain:
 *                 type: string
 *                 description: Home domain for the challenge
 *               client_domain:
 *                 type: string
 *                 description: Client domain
 *     responses:
 *       200:
 *         description: Challenge transaction generated
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/AuthChallenge'
 *       400:
 *         description: Invalid request parameters
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/Error'
 */
router.post('/', async (req: Request, res: Response) => {
  return getChallenge(req, res, redisService);
});

/**
 * @swagger
 * /auth/token:
 *   post:
 *     summary: SEP-10 Token Endpoint
 *     description: Validates a signed challenge transaction and returns a JWT token
 *     tags: [Auth]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required:
 *               - transaction
 *             properties:
 *               transaction:
 *                 type: string
 *                 description: Signed SEP-10 challenge transaction XDR
 *               client_signature:
 *                 type: string
 *                 description: Client signature
 *     responses:
 *       200:
 *         description: Authentication successful
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/AuthToken'
 *       400:
 *         description: Invalid or expired challenge
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/Error'
 *       401:
 *         description: Invalid signature
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/Error'
 */
router.post('/token', async (req: Request, res: Response) => {
  return getToken(req, res, redisService);
});

export default router;