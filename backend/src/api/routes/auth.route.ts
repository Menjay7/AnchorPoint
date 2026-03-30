import { Router, Request, Response } from 'express';
import { RedisService } from '../../services/redis.service';
import { getChallenge, getToken } from '../controllers/auth.controller';

const router = Router();

// Mock Redis client for demonstration
// In a real implementation, you would inject the actual Redis client
const mockRedisClient = {
  get: async () => null,
  set: async () => {},
  del: async () => 1,
  expire: async () => {}
};

const redisService = new RedisService(mockRedisClient);

/**
 * POST /auth
 * SEP-10 Challenge Endpoint
 */
router.post('/', async (req: Request, res: Response) => {
  return getChallenge(req, res, redisService);
});

/**
 * POST /auth/token
 * SEP-10 Token Endpoint
 */
router.post('/token', async (req: Request, res: Response) => {
  return getToken(req, res, redisService);
});

export default router;
