// Data tables for check-deps.mjs. Mirrors ARCHITECTURE §4.5 cell by cell.
// Rows = "from" crate, columns = "to". `true` = allowed (✓), `false` = forbidden (✗).
// The doc table only lists 7 columns (protocol, sim, sim-wasm, server, gateway, bots, client);
// headless-client and balance-sim have no column, which means nothing may depend on them
// (§4.2 draws no edge into them), so they are forbidden targets here.
// "client" is the TypeScript package and is not a Rust crate; the column is kept so the table
// stays identical to §4.5, and any workspace crate named `frontline-client` would be rejected.

export const CRATES = [
  'protocol',
  'sim',
  'sim-wasm',
  'server',
  'gateway',
  'bots',
  'headless-client',
  'balance-sim',
];

export const COLUMNS = [...CRATES, 'client'];

// Allowed targets per source crate (every ✓ in §4.5; anything not listed is ✗).
export const ALLOWED = {
  protocol: [],
  sim: ['protocol'],
  'sim-wasm': ['protocol', 'sim'],
  server: ['protocol', 'sim', 'bots'],
  gateway: ['protocol'],
  bots: ['protocol', 'sim'],
  'headless-client': ['protocol', 'sim'],
  'balance-sim': ['protocol', 'sim', 'bots'],
};

// Cargo package name = `${PACKAGE_PREFIX}${short name}`.
export const PACKAGE_PREFIX = 'frontline-';

// Crates that must not (transitively) depend on these packages. §4.3.1 / §4.3.2.
// `protocol`'s `std` feature gets no exemption: none of these crates is needed even with std.
export const TRANSITIVE_FORBIDDEN = {
  sim: ['tokio', 'wasm-bindgen', 'wtransport', 'rayon'],
  protocol: ['tokio', 'wasm-bindgen', 'wtransport', 'rayon'],
};
