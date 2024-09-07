## XA Decoder

A XA audio decoder written in Rust with Wasm. 
Original implementation from https://github.com/dridi/bjxa/.

### Usage
```ts
import init, { WasmXADecoder } from "xa_decoder";

let audioContext = new (window.AudioContext || window.webkitAudioContext)();

async function decodeAudioFile(file: File) {
    await init();
    const decoder = new WasmXADecoder();
    const arrayBuffer = await file.arrayBuffer();
    const data = decoder.decode(new Uint8Array(arrayBuffer));
    const format = decoder.get_format();

    const audioBuffer = audioContext.createBuffer(format.channels, data.length, format.samples_rate);
    for (let i = 0; i < format.channels; i++) {
        const channelData = audioBuffer.getChannelData(i);
        for (let j = 0; j < data.length; j++) {
            channelData[j] = data[j] / 32768;
        }
    }
    const source = audioContext.createBufferSource();
    source.buffer = audioBuffer;
    source.connect(audioContext.destination);
    source.start();
}
```

### Build
```
wasm-pack build --target web
```

### Start the testing web server

```
python3 -m http.server
```