import { Circomkit } from "circomkit";

export const circomkit = new Circomkit({
  verbose: false,
});

export function randomBigInt(): bigint {
  return BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER));
}