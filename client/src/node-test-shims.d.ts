// Minimal typings for the few Node built-ins used by vitest files (the client does not depend on @types/node;
// browser code must not see Node globals). Extend here if another test needs more.
declare module 'node:fs' {
  export function existsSync(path: string): boolean;
  export function readFileSync(path: string): Uint8Array;
  export function readFileSync(path: string, encoding: 'utf8'): string;
}
declare module 'node:crypto' {
  export function createHash(algorithm: string): { update(data: Uint8Array | string): { digest(encoding: 'hex'): string } };
}
declare module 'node:url' {
  export function fileURLToPath(url: string | URL): string;
}
