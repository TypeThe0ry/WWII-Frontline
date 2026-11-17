// Server lifecycle for the interop run. The processes, certificate and harness page are the M0-021 helpers (../net-support.ts);
// this file only fixes the interop configuration: map m0, mode sandbox, 0 bots (the defaults of frontline-server).
// Snapshot source: until M0-019 (AOI + snapshot encoding in sim_thread) lands the server sends no C2 snapshots by itself;
// `--dev-echo` (C1 bundle -> C2 packet, M0-017 scaffold) is then the only downstream traffic, and `ServerProc.echo` says
// whether it was accepted. The probe records which source it counted (ProbeResult.snapshotSource).
import { type DevCert, type ServerProc, startServer } from '../net-support';

/** WebTransport + WebSocket when a certificate is given, WebSocket only otherwise. */
export function startInteropServer(cert?: DevCert): Promise<ServerProc> {
  return startServer({ cert });
}
