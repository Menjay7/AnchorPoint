import { Response } from 'express';
import { randomUUID } from 'crypto';
import { AuthRequest } from '../middleware/auth.middleware';
import * as sep6Controller from './sep6.controller';
import * as kycService from '../../services/kyc.service';
import prisma from '../../lib/prisma';
import { ASSETS } from '../../config/assets';

jest.mock('../../lib/prisma');
jest.mock('../../services/kyc.service');
jest.mock('crypto');

describe('SEP-6 Controller', () => {
  let mockRequest: Partial<AuthRequest>;
  let mockResponse: Partial<Response>;
  let jsonMock: jest.Mock;
  let statusMock: jest.Mock;

  beforeEach(() => {
    jsonMock = jest.fn().mockReturnThis();
    statusMock = jest.fn().mockReturnValue({ json: jsonMock });

    mockRequest = {
      body: {},
      query: {},
      user: { publicKey: 'GBTEST123' }
    };

    mockResponse = {
      json: jsonMock,
      status: statusMock
    };

    jest.clearAllMocks();
    (randomUUID as jest.Mock).mockReturnValue('test-transaction-id-123');
  });

  describe('sep6Info', () => {
    it('should return info with supported assets', () => {
      sep6Controller.sep6Info(mockRequest as AuthRequest, mockResponse as Response);

      expect(jsonMock).toHaveBeenCalled();
      const response = jsonMock.mock.calls[0][0];
      expect(response).toHaveProperty('deposit');
      expect(response).toHaveProperty('withdraw');
    });

    it('should include deposit info for supported deposit assets', () => {
      sep6Controller.sep6Info(mockRequest as AuthRequest, mockResponse as Response);

      const response = jsonMock.mock.calls[0][0];
      expect(Object.keys(response.deposit).length).toBeGreaterThan(0);
      const firstAsset = Object.values(response.deposit)[0] as any;
      expect(firstAsset).toHaveProperty('enabled', true);
      expect(firstAsset).toHaveProperty('min_amount');
      expect(firstAsset).toHaveProperty('max_amount');
      expect(firstAsset).toHaveProperty('fee_fixed');
      expect(firstAsset).toHaveProperty('fee_percent');
      expect(firstAsset).toHaveProperty('fields');
    });

    it('should include withdraw info for supported withdraw assets', () => {
      sep6Controller.sep6Info(mockRequest as AuthRequest, mockResponse as Response);

      const response = jsonMock.mock.calls[0][0];
      expect(Object.keys(response.withdraw).length).toBeGreaterThan(0);
      const firstAsset = Object.values(response.withdraw)[0] as any;
      expect(firstAsset).toHaveProperty('enabled', true);
      expect(firstAsset).toHaveProperty('types');
    });
  });

  describe('sep6Deposit', () => {
    beforeEach(() => {
      (kycService.isDepositSupported as jest.Mock).mockReturnValue(true);
      (kycService.normalizeAssetCode as jest.Mock).mockImplementation(code => code.toUpperCase());
      (kycService.getAsset as jest.Mock).mockReturnValue({
        code: 'USDC',
        minAmount: '0.01',
        maxAmount: '100000',
        feeFixed: '0.1',
        feePercent: '0.001'
      });
    });

    it('should return 400 when asset_code is missing', async () => {
      mockRequest.query = {};

      await sep6Controller.sep6Deposit(mockRequest as AuthRequest, mockResponse as Response);

      expect(statusMock).toHaveBeenCalledWith(400);
      expect(jsonMock).toHaveBeenCalledWith(
        expect.objectContaining({
          error: expect.any(String)
        })
      );
    });

    it('should return 400 for unsupported asset', async () => {
      mockRequest.query = { asset_code: 'UNSUPPORTED' };
      (kycService.isDepositSupported as jest.Mock).mockReturnValue(false);

      await sep6Controller.sep6Deposit(mockRequest as AuthRequest, mockResponse as Response);

      expect(statusMock).toHaveBeenCalledWith(400);
      expect(jsonMock).toHaveBeenCalledWith(
        expect.objectContaining({
          error: expect.stringContaining('not supported for deposit')
        })
      );
    });

    it('should return 400 when amount is below minimum', async () => {
      mockRequest.query = { asset_code: 'USDC', amount: '0.001' };

      await sep6Controller.sep6Deposit(mockRequest as AuthRequest, mockResponse as Response);

      expect(statusMock).toHaveBeenCalledWith(400);
      expect(jsonMock).toHaveBeenCalledWith(
        expect.objectContaining({
          error: expect.stringContaining('must be between')
        })
      );
    });

    it('should return 400 when amount is above maximum', async () => {
      mockRequest.query = { asset_code: 'USDC', amount: '1000000' };

      await sep6Controller.sep6Deposit(mockRequest as AuthRequest, mockResponse as Response);

      expect(statusMock).toHaveBeenCalledWith(400);
      expect(jsonMock).toHaveBeenCalledWith(
        expect.objectContaining({
          error: expect.stringContaining('must be between')
        })
      );
    });

    it('should create deposit transaction successfully', async () => {
      mockRequest.query = { asset_code: 'USDC', amount: '100', email_address: 'user@example.com' };
      (prisma.user.upsert as jest.Mock).mockResolvedValue({ id: 'user-1', publicKey: 'GBTEST123' });
      (prisma.transaction.create as jest.Mock).mockResolvedValue({
        id: 'test-transaction-id-123',
        userId: 'user-1',
        assetCode: 'USDC',
        amount: '100',
        type: 'DEPOSIT',
        status: 'PENDING'
      });

      await sep6Controller.sep6Deposit(mockRequest as AuthRequest, mockResponse as Response);

      expect(jsonMock).toHaveBeenCalledWith(
        expect.objectContaining({
          id: 'test-transaction-id-123',
          eta: expect.any(Number),
          min_amount: '0.01',
          max_amount: '100000'
        })
      );
    });

    it('should normalize asset code to uppercase', async () => {
      mockRequest.query = { asset_code: 'usdc', amount: '100' };
      (prisma.user.upsert as jest.Mock).mockResolvedValue({ id: 'user-1' });
      (prisma.transaction.create as jest.Mock).mockResolvedValue({ id: 'tx-1' });

      await sep6Controller.sep6Deposit(mockRequest as AuthRequest, mockResponse as Response);

      expect(kycService.normalizeAssetCode).toHaveBeenCalledWith('usdc');
    });
  });
});
