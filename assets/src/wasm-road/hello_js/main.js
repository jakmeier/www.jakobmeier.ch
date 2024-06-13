async function fetchAndRunWasm() {
    // Function will be overwritten once we have a memory instance to access.
    let readWasmString = (_memory) => "";

    // functions to inject into Wasm
    const imports = {
        print_char: (c) => {
            const char = String.fromCharCode(c);
            console.log(char);
        },
        print_string: (start, len) => {
            const msg = readWasmString(start, len);
            console.log(msg);
        }
    };

    // efficiently download and load Wasm module instance
    const wasm = await WebAssembly.instantiateStreaming(fetch('hello_js.wasm'), {
        env: imports,
    });

    // Get the exports from the WASM module, functions and memory
    const { hello_world, hello_world_2, memory } = wasm.instance.exports;

    // overwrite `readWasmString` to access the correct Wasm memory buffer
    readWasmString = (start, len) => {
        // Read data directly from Wasm memory
        const buf = new Uint8Array(memory.buffer, start, len);
        return new TextDecoder('utf8').decode(buf);
    }

    // Call the exported functions
    hello_world();
    hello_world_2();
}

fetchAndRunWasm();