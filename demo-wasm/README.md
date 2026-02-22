# demo-wasm

Browser demo for `cogwise`, designed for GitHub Pages deployment.

## Local Build

```bash
cd demo-wasm
wasm-pack build --target web --release
```

Serve `demo-wasm/www` with a local static server after build, and ensure
`demo-wasm/www/main.js` can load `../pkg/cogwise_demo_wasm.js`.
