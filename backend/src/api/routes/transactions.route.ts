import { Router, Response } from 'express';
import { z } from 'zod';
import prisma from '../../lib/prisma';
import { authMiddleware, AuthRequest } from '../middleware/auth.middleware';
import { validate } from '../middleware/validate.middleware';

const router = Router();

const querySchema = z.object({
  page: z.string().optional().transform(v => parseInt(v || '1', 10)).pipe(z.number().min(1)),
  limit: z.string().optional().transform(v => parseInt(v || '10', 10)).pipe(z.number().min(1).max(50)),
  assetCode: z.string().optional(),
});

/**
 * GET /api/transactions
 * Fetches transaction history for the authenticated user.
 */
router.get('/', authMiddleware, validate({ query: querySchema }), async (req: AuthRequest, res: Response) => {
  const { page, limit, assetCode } = req.query as unknown as {
    page: number;
    limit: number;
    assetCode?: string;
  };
  const publicKey = req.user!.publicKey;

  try {
    const skip = (page - 1) * limit;

    const [transactions, total] = await Promise.all([
      prisma.transaction.findMany({
        where: {
          user: {
            publicKey,
          },
          ...(assetCode && { assetCode }),
        },
        orderBy: {
          createdAt: 'desc',
        },
        skip,
        take: limit,
      }),
      prisma.transaction.count({
        where: {
          user: {
            publicKey,
          },
          ...(assetCode && { assetCode }),
        },
      }),
    ]);

    res.json({
      status: 'success',
      data: {
        transactions,
        pagination: {
          total,
          page,
          limit,
          totalPages: Math.ceil(total / limit),
        },
      },
    });
  } catch (error) {
    console.error('Error fetching transactions:', error);
    res.status(500).json({
      status: 'error',
      message: 'Failed to fetch transaction history',
    });
  }
});

export default router;
