// Server lifecycle for the interop run. The processes, certificate and harness page are the M0-021 helpers (../net-support.ts);
// this file only fixes the interop configuration: map m0, mode sandbox, 0 bots (the defaults of frontline-server) and the
// real simulation snapshots (never `--dev-echo`, which would make the snapshot count meaningless).
import { type DevCert, type ServerProc, startServer } from '../net-support';

/** WebTransport + WebSocket when a certificate is given, WebSocket only otherwise. */
export function startInteropServer(cert?: DevCert): Promise<ServerProc> {
  return startServer({ cert, echo: false });
}
