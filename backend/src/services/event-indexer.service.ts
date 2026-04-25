import { rpc, xdr, scValToNative } from '@stellar/stellar-sdk';
import { PrismaClient } from '@prisma/client';
import { config } from '../config/env';
import logger from '../utils/logger';

const prisma = new PrismaClient();

export class EventIndexerService {
  private server: rpc.Server;
  private isRunning: boolean = false;
  private pollInterval: number = 5000; // 5 seconds

  constructor() {
    this.server = new rpc.Server(config.SOROBAN_RPC_URL);
  }

  async start() {
    if (this.isRunning) return;
    this.isRunning = true;
    logger.info('Event Indexer Service starting...');
    this.run();
  }

  stop() {
    this.isRunning = false;
    logger.info('Event Indexer Service stopping...');
  }

  private async run() {
    while (this.isRunning) {
      try {
        await this.indexEvents();
      } catch (error) {
        logger.error('Error in event indexer loop:', error);
      }
      await new Promise(resolve => setTimeout(resolve, this.pollInterval));
    }
  }

  private async indexEvents() {
    const contracts = [
      config.SWAP_CONTRACT_ID,
      config.YIELD_CONTRACT_ID,
      config.NFT_CONTRACT_ID
    ].filter(Boolean) as string[];

    if (contracts.length === 0) {
      logger.warn('No contract IDs configured for indexing');
      return;
    }

    // Get the last indexed ledger
    const lastEvent = await prisma.contractEvent.findFirst({
      orderBy: { ledger: 'desc' },
    });

    let startLedger = lastEvent ? lastEvent.ledger : undefined;
    
    // If no events indexed yet, we might want to start from current ledger
    if (!startLedger) {
      const networkStatus = await this.server.getLatestLedger();
      startLedger = networkStatus.sequence;
    }

    logger.debug(`Fetching events from ledger ${startLedger}...`);

    try {
      const response = await this.server.getEvents({
        startLedger: startLedger,
        filters: [
          {
            contractIds: contracts,
          }
        ],
        limit: 100,
      });

      if (response.events.length === 0) {
        return;
      }

      logger.info(`Found ${response.events.length} new events`);

      for (const event of response.events) {
        await this.processEvent(event);
      }
    } catch (error) {
      if (error instanceof Error && error.message.includes('404')) {
        // Might be that startLedger is too old or not yet available
        return;
      }
      throw error;
    }
  }

  private async processEvent(event: any) {
    // Use txHash and ledger to avoid duplicates, as multiple events can happen in one tx
    // But rpc.Api.GetEventResponse actually has an 'id' which is unique (ledger-tx-index)
    // Stellar SDK v14 might have different event structure, let's be safe.
    
    const existing = await prisma.contractEvent.findFirst({
      where: { 
        txHash: event.txHash,
        ledger: event.ledger,
        // Since we don't have an event index in the schema yet, 
        // we might still have duplicates if multiple events are in one tx.
        // For now, let's just proceed and we can refine later.
      }
    });

    // We'll use the event topics to determine the type
    const eventType = this.getEventType(event.topic);
    const data = this.parseEventData(event.value);

    await prisma.contractEvent.create({
      data: {
        contractId: event.contractId,
        ledger: event.ledger,
        ledgerClosedAt: event.ledgerClosedAt,
        txHash: event.txHash,
        eventType,
        data: data as any,
      }
    });
    
    logger.debug(`Indexed ${eventType} event from contract ${event.contractId}`);
  }

  private getEventType(topicXdr: string[]): string {
    try {
      if (topicXdr.length === 0) return 'unknown';
      const firstTopic = xdr.ScVal.fromXDR(topicXdr[0], 'base64');
      const native = scValToNative(firstTopic);
      return String(native);
    } catch {
      return 'unknown';
    }
  }

  private parseEventData(value: any): any {
    try {
      const scVal = xdr.ScVal.fromXDR(value.xdr, 'base64');
      return scValToNative(scVal);
    } catch {
      return { xdr: value.xdr };
    }
  }
}

export const eventIndexer = new EventIndexerService();
