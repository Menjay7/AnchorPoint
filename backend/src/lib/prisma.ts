import { PrismaClient } from '@prisma/client';

const basePrisma = new PrismaClient();

const prisma = basePrisma
  .$extends({
    query: {
      $allModels: {
        async $allOperations({ operation, model, args, query }) {
          const start = performance.now();
          const result = await query(args);
          const time = performance.now() - start;
          if (time > 50) {
            console.warn(`[PRISMA SLOW QUERY] ${model}.${operation} took ${time.toFixed(2)}ms`);
          }
          return result;
        },
        async findMany({ args, query }) {
          args.where = { deletedAt: null, ...args.where };
          return query(args);
        },
        async findFirst({ args, query }) {
          args.where = { deletedAt: null, ...args.where };
          return query(args);
        },
        async findFirstOrThrow({ args, query }) {
          args.where = { deletedAt: null, ...args.where };
          return query(args);
        },
        async count({ args, query }) {
          if (args) {
            args.where = { deletedAt: null, ...args.where };
          } else {
            args = { where: { deletedAt: null } };
          }
          return query(args);
        },
        async delete({ model, args }) {
          return (basePrisma as any)[model].update({
            ...args,
            data: { deletedAt: new Date() },
          });
        },
        async deleteMany({ model, args }) {
          if (args?.where) {
            args.where = { deletedAt: null, ...args.where };
          } else if (args) {
            args.where = { deletedAt: null };
          } else {
            args = { where: { deletedAt: null } };
          }
          return (basePrisma as any)[model].updateMany({
            ...args,
            data: { deletedAt: new Date() },
          });
        }
      }
    }
  });

export default prisma;
