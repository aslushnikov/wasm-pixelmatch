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

Measurement: comparing red 1280 x 720 image with white 1280 x 720 images; threshold = 2, the rest is default.

```
pixelmatch: 67.481ms
wasm.pixelmatch: 45.518ms
```


