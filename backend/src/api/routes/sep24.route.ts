import { Router } from 'express';
import { depositInteractive, withdrawInteractive } from '../controllers/sep24.controller';

const router = Router();

/**
 * POST /transactions/deposit/interactive
 * SEP-24 Interactive Deposit Endpoint
 */
router.post('/transactions/deposit/interactive', depositInteractive);

/**
 * POST /transactions/withdraw/interactive
 * SEP-24 Interactive Withdraw Endpoint
 */
router.post('/transactions/withdraw/interactive', withdrawInteractive);

export default router;
