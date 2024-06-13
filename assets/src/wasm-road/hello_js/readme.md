# Hello JS from Wasm

A simple demo for imports and exports in Rust, without any bindgen magic.

To run it locally, you probably need a simple server to avoid CORS issues that
happen when you just open the HTML in the browser.

For example, if you have python:

```bash
python3 -m http.server 8456
```

Then open http://0.0.0.0:8456/ and check the console.