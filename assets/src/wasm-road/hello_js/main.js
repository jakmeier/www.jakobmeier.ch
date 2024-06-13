async function fetchAndRunWasm() {
    const wasmModule = await WebAssembly.instantiateStreaming(fetch('hello_js.wasm'), {
        env: {
            print_char: (c) => {
                const char = String.fromCharCode(c);
                console.log(char);
            }
        }
    });

    // Get the exports from the WASM module
    const { hello_world } = wasmModule.instance.exports;

    // Call the exported function
    hello_world();
}

fetchAndRunWasm();