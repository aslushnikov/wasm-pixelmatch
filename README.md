# Wasm Pixelmatch

This is an experimental port of https://github.com/mapbox/pixelmatch to Rust and Wasm.

Building:

```bash
wasm-pack build --target nodejs
```

Using:

```js
const { pixelmatch } = require('./pkg/wasm_pixelmatch.js');
// ...
```
