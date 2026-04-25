import * as StellarSdk from '@stellar/stellar-sdk';
import { config } from '../config/env';
import logger from '../utils/logger';

/**
 * Service for handling Stellar blockchain operations
 */
export class StellarService {
  private server: StellarSdk.Horizon.Server;
  private networkPassphrase: string;
  private feeBumpKeypair: StellarSdk.Keypair | null = null;

  // Whitelisted operations for submission
  private readonly ALLOWED_OPERATIONS = [
    'payment',
    'changeTrust',
    'manageData',
    'setOptions',
    'manageBuyOffer',
    'manageSellOffer',
    'createAccount',
  ];

  constructor() {
    this.server = new StellarSdk.Horizon.Server(config.STELLAR_HORIZON_URL);
    this.networkPassphrase = config.STELLAR_NETWORK === 'public'
      ? StellarSdk.Networks.PUBLIC
      : StellarSdk.Networks.TESTNET;

    if (config.STELLAR_FEE_BUMP_SECRET) {
      this.feeBumpKeypair = StellarSdk.Keypair.fromSecret(config.STELLAR_FEE_BUMP_SECRET);
    }
  }

  /**
   * Validates and submits a pre-signed transaction XDR
   * @param xdr Base64 encoded transaction XDR
   * @returns Submission result
   */
  async submitTransaction(xdr: string): Promise<any> {
    try {
      const tx = StellarSdk.TransactionBuilder.fromXDR(xdr, this.networkPassphrase);
      
      // Ensure it's not a fee-bump transaction itself
      if (tx instanceof StellarSdk.FeeBumpTransaction) {
        throw new Error('Direct submission of fee-bump transactions is not allowed');
      }

      // Now tx is guaranteed to be a regular Transaction
      const transaction = tx as StellarSdk.Transaction;

      // Validate operations against whitelist
      this.validateOperations(transaction);

      // Automated Fee Management: Wrap in a fee-bump transaction if backend is configured
      let finalTx: StellarSdk.Transaction | StellarSdk.FeeBumpTransaction = transaction;
      
      if (this.feeBumpKeypair) {
        logger.info(`Applying fee-bump for transaction from ${transaction.source}`);
        finalTx = StellarSdk.TransactionBuilder.buildFeeBumpTransaction(
          this.feeBumpKeypair,
          config.STELLAR_BASE_FEE,
          transaction,
          this.networkPassphrase
        );
      }


      const response = await this.server.submitTransaction(finalTx);
      logger.info(`Transaction submitted successfully: ${response.hash}`);
      return response;
    } catch (error: any) {
      const errorMessage = error.response?.data?.extras?.result_codes?.operations 
        ? `Stellar Error: ${JSON.stringify(error.response.data.extras.result_codes)}`
        : error.message;
      
      logger.error('Stellar submission error:', errorMessage);
      throw new Error(errorMessage);
    }
  }

  /**
   * Validates that all operations in the transaction are whitelisted
   */
  private validateOperations(tx: StellarSdk.Transaction): void {
    for (const op of tx.operations) {
      if (!this.ALLOWED_OPERATIONS.includes(op.type)) {
        throw new Error(`Operation type '${op.type}' is not whitelisted for this endpoint`);
      }
    }
  }

  /**
   * Helper to extract source account from XDR without full validation
   */
  static getSourceAccountFromXDR(xdr: string): string {
    try {
      const tx = StellarSdk.TransactionBuilder.fromXDR(xdr, StellarSdk.Networks.TESTNET);
      if (tx instanceof StellarSdk.FeeBumpTransaction) {
        return tx.innerTransaction.source;
      }
      return tx.source;
    } catch (error) {
      throw new Error('Invalid transaction XDR');
    }

  }
}

export const stellarService = new StellarService();
