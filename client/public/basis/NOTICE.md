# Basis Universal transcoder (vendored)

`basis_transcoder.js` / `basis_transcoder.wasm` are the prebuilt Emscripten build of the Binomial
[Basis Universal](https://github.com/BinomialLLC/basis_universal) KTX2/Basis transcoder, licensed **Apache-2.0**
(upstream `LICENSE`; the copyright holder is Binomial LLC). They were taken unmodified from the `three@0.187.0` npm package
(`examples/jsm/libs/basis/`, `three` itself is MIT, see `LICENSE-three.txt`) on 2026-11-17 (M1-006c, PLAN_ISSUES PI-044).

SHA-256: see LICENSES.md section 5. Loaded lazily by the decode Worker (`client/src/render/assets/transcode.ts`); not part
of the `core` bundle.
