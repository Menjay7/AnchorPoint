import express, { Request, Response, NextFunction } from 'express';
import cors from 'cors';
import swaggerUi from 'swagger-ui-express';
import { config } from './config/env';
import { swaggerSpec } from './config/swagger';
import logger from './utils/logger';
import transactionsRouter from './api/routes/transactions.route';
import sep24Router from './api/routes/sep24.route';
import infoRouter from './api/routes/info.route';
import { errorHandler } from './api/middleware/error.middleware';

const app = express();
const PORT = config.PORT;

app.use(cors());
app.use(express.json());

// Swagger API Documentation
/**
 * @swagger
 * /api-docs:
 *   get:
 *     summary: API Documentation
 *     description: Interactive Swagger UI documentation for the AnchorPoint API
 *     tags: [Documentation]
 *     responses:
 *       200:
 *         description: Swagger UI HTML page
 */
app.use('/api-docs', swaggerUi.serve, swaggerUi.setup(swaggerSpec, {
  customCss: '.swagger-ui .topbar { display: none }',
  customSiteTitle: 'AnchorPoint API Documentation',
  swaggerOptions: {
    persistAuthorization: true,
    displayOperationId: true,
    filter: true,
  },
}));

// API Documentation JSON endpoint
app.get('/api-docs.json', (req: Request, res: Response) => {
  res.setHeader('Content-Type', 'application/json');
  res.send(swaggerSpec);
});

app.use('/api/transactions', transactionsRouter);

// SEP-1 Info endpoint
app.use('/info', infoRouter);

// SEP-24 routes
app.use('/sep24', sep24Router);

/**
 * @swagger
 * /health:
 *   get:
 *     summary: Health check
 *     description: Check if the API server is running
 *     tags: [Health]
 *     responses:
 *       200:
 *         description: Server is healthy
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 status:
 *                   type: string
 *                   example: UP
 *                 timestamp:
 *                   type: string
 *                   format: date-time
 */
app.get('/health', (req: Request, res: Response) => {
  res.json({ status: 'UP', timestamp: new Date().toISOString() });
});

/**
 * @swagger
 * /:
 *   get:
 *     summary: Root endpoint
 *     description: Welcome message for the AnchorPoint API
 *     tags: [Health]
 *     responses:
 *       200:
 *         description: Welcome message
 *         content:
 *           text/html:
 *             schema:
 *               type: string
 *               example: AnchorPoint Backend API is running.
 */
app.get('/', (req: Request, res: Response) => {
  res.send('AnchorPoint Backend API is running.');
});

// Global error handling middleware (must be last)
app.use(errorHandler);

/* istanbul ignore next */
if (process.env.NODE_ENV !== 'test') {
  app.listen(PORT, () => {
    logger.info(`Backend service listening at http://localhost:${PORT}`);
    logger.info(`API Documentation available at http://localhost:${PORT}/api-docs`);
  });
}

export default app;
