import { afterEach, expect, it, vi } from 'vitest';
import { GatewayClient } from './gateway-client';

const ID = '0123456789abcdef0123456789abcdef';
const valid = { match_id: ID, name: 'Alpha', map: 'm0', mode: 'sandbox', phase: 'live', region: 'local', version: 'dev', compatible: false, humans: 3, bots: 4, cap: 100 };
afterEach(() => vi.restoreAllMocks());

it('maps the actual gateway list without inventing ping or password support', async () => {
  const fetcher = vi.fn<typeof fetch>().mockImplementation(async () => Response.json([valid]));
  const client = new GatewayClient('https://gateway.test/', fetcher);
  const [entry] = await client.list();
  expect(fetcher).toHaveBeenCalledWith('https://gateway.test/v1/servers', expect.objectContaining({ method: 'GET', redirect: 'error' }));
  expect(entry).toMatchObject({ id: ID, compatible: false, version: 'dev', humans: 3 });
  expect(entry?.pingMs).toBeUndefined();
  expect(entry?.locked).toBeUndefined();
  fetcher.mockResolvedValueOnce(Response.json([{ ...valid, cap: '100' }]));
  await expect(client.list()).rejects.toMatchObject({ code: 'invalid_response' });
});

it('configures a 5000 ms deadline that includes waiting for the body', async () => {
  const deadline = new AbortController();
  const timeout = vi.spyOn(AbortSignal, 'timeout').mockReturnValue(deadline.signal);
  const fetcher: typeof fetch = async (_url, opts) => new Response(new ReadableStream({ start(controller) {
    opts?.signal?.addEventListener('abort', () => controller.error(opts.signal?.reason));
  } }));
  const list = new GatewayClient('', fetcher).list();
  expect(timeout).toHaveBeenCalledWith(5_000);
  await Promise.resolve();
  deadline.abort(new DOMException('fixture timeout', 'TimeoutError'));
  await expect(list).rejects.toMatchObject({ code: 'timeout' });
});

it('uses the ticket error code and rejects invalid server addresses or match IDs', async () => {
  const fetcher = vi.fn<typeof fetch>().mockImplementation(async () => Response.json({ error: 'match_full' }, { status: 409 }));
  const client = new GatewayClient('', fetcher);
  const guest = { token: 'fixture-token', nick: 'Guest-0000' };
  await expect(client.join('../other', guest, '')).rejects.toMatchObject({ code: 'invalid_match' });
  expect(fetcher).not.toHaveBeenCalled();
  await expect(client.join(ID, guest, '')).rejects.toMatchObject({ code: 'match_full', status: 409 });
  expect(fetcher).toHaveBeenCalledWith(`/v1/servers/${ID}/ticket`, expect.objectContaining({
    headers: { Authorization: 'Bearer fixture-token', 'Content-Type': 'application/json' }, body: '{"nick":"Guest-0000"}',
  }));
  const ticket = { ticket: 'claims.signature', addr: '2001:db8::1', ws_port: 7000, wt_port: 7001 };
  fetcher.mockResolvedValueOnce(Response.json(ticket));
  await expect(client.join(ID, guest, ' Custom ')).resolves.toEqual({ ticket: ticket.ticket, host: '[2001:db8::1]', wsPort: 7000, wtPort: 7001 });
  fetcher.mockResolvedValueOnce(Response.json({ ...ticket, addr: 'evil.example@localhost' }));
  await expect(client.join(ID, guest, '')).rejects.toMatchObject({ code: 'invalid_response' });
});
