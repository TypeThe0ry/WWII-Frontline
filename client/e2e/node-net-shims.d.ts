// Minimal typings for the Node built-ins used by net-support.ts (the client has no @types/node; see src/node-test-shims.d.ts).
declare module 'node:child_process' {
  export interface ChildProcess {
    readonly stdout: { on(ev: 'data', cb: (d: { toString(): string }) => void): void } | null;
    readonly stderr: { on(ev: 'data', cb: (d: { toString(): string }) => void): void } | null;
    kill(signal?: string): boolean;
    once(ev: 'exit', cb: () => void): void;
  }
  export function spawn(cmd: string, args: string[], opts: { stdio: string[]; cwd?: string }): ChildProcess;
  export function execFileSync(cmd: string, args: string[], opts?: { stdio?: string }): unknown;
}
declare module 'node:fs' {
  export function mkdtempSync(prefix: string): string;
  export function writeFileSync(path: string, data: string): void;
  export function rmSync(path: string, opts?: { recursive?: boolean; force?: boolean }): void;
}
declare module 'node:dgram' {
  export function createSocket(type: 'udp4'): {
    once(ev: 'error', cb: (e: Error) => void): void;
    on(ev: 'message', cb: () => void): void;
    bind(port: number, host: string, cb: () => void): void;
    address(): { port: number };
    close(cb: () => void): void;
  };
}
declare module 'node:net' {
  export function createServer(): {
    once(ev: 'error', cb: (e: Error) => void): void;
    listen(port: number, host: string, cb: () => void): void;
    address(): { port: number } | string | null;
    close(cb: () => void): void;
  };
}
declare module 'node:os' {
  export function tmpdir(): string;
}
declare module 'node:path' {
  export function join(...parts: string[]): string;
  export function resolve(...parts: string[]): string;
}
