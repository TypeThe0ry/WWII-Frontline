/** Thin ffmpeg subprocess wrapper. ffmpeg is a build-machine tool only: its binary is never shipped (see LICENSES.md section 5). */
import { spawn } from 'node:child_process';
import { ExitCode, GateError } from '../errors.ts';

export interface FfmpegResult {
  stdout: Buffer;
  stderr: string;
}

export function ffmpeg(args: string[], opts: { input?: Uint8Array } = {}): Promise<FfmpegResult> {
  return new Promise((resolve, reject) => {
    const p = spawn('ffmpeg', ['-hide_banner', '-nostdin', '-y', ...args], { stdio: ['pipe', 'pipe', 'pipe'] });
    const out: Buffer[] = [];
    let err = '';
    p.stdout.on('data', (d: Buffer) => out.push(d));
    p.stderr.on('data', (d: Buffer) => (err += d.toString()));
    p.on('error', (e) => reject(new GateError(ExitCode.Usage, `ffmpeg could not be started (${e.message}); install ffmpeg on the build machine`)));
    p.on('close', (code) => {
      if (code === 0) resolve({ stdout: Buffer.concat(out), stderr: err });
      else reject(new GateError(ExitCode.Usage, `ffmpeg ${args.slice(0, 6).join(' ')} ... failed (${code}): ${err.split('\n').slice(-4).join(' | ')}`));
    });
    p.stdin.on('error', () => undefined);
    p.stdin.end(opts.input ? Buffer.from(opts.input) : undefined);
  });
}
