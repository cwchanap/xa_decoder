import init, { WasmXADecoder } from './pkg/xa_decoder.js';

let audioContext;

async function decodeXA(xaData) {
    const decoder = new WasmXADecoder();
    let format;
    try {
        format = decoder.read_header(new Uint8Array(xaData));
        console.log(format);
    } catch (error) {
        console.error('Error reading XA header:', error);
        throw error;
    }

    try {
        const pcmData = decoder.decode(new Uint8Array(xaData));
        console.log(pcmData);
    } catch (error) {
        console.error('Error during decoding:', error);
        console.error("Error stack:", error.stack);
        throw error;
    }

    return { format, pcmData };
}

function convertToAudioBuffer(pcmData, sampleRate) {
    const audioBuffer = audioContext.createBuffer(1, pcmData.length, sampleRate);
    const channelData = audioBuffer.getChannelData(0);
    for (let i = 0; i < pcmData.length; i++) {
        channelData[i] = pcmData[i] / 32768;
    }
    return audioBuffer;
}

function playAudio(audioBuffer) {
    const source = audioContext.createBufferSource();
    source.buffer = audioBuffer;
    source.connect(audioContext.destination);
    source.start();
}

async function decodeFile(file) {
    try {
        const arrayBuffer = await file.arrayBuffer();
        const { format, pcmData } = await decodeXA(arrayBuffer);
        const audioBuffer = convertToAudioBuffer(pcmData, format.samples_rate);
        return { format, pcmData, audioBuffer };
    } catch (error) {
        console.error(`Error decoding file ${file.name}:`, error);
        throw error;
    }
}

async function decodeAllFiles() {
    const resultsDiv = document.getElementById('results');
    resultsDiv.innerHTML = '';

    const inputs = ['bassInput', 'highTomInput', 'midTomInput', 'lowTomInput', 'hiHatInput'];

    for (const inputId of inputs) {
        const input = document.getElementById(inputId);
        if (!input.files[0]) continue;

        try {
            const { format, pcmData, audioBuffer } = await decodeFile(input.files[0]);

            const first200Samples = Array.from(pcmData.slice(0, 200));
            const resultHtml = `
                <h2>${inputId}</h2>
                <p>Sample Rate: ${format.samples_rate} Hz</p>
                <p>Channels: ${format.channels}</p>
                <p>PCM data (first 200 samples): ${JSON.stringify(first200Samples)}</p>
                <button onclick="window.playAudio('${inputId}')">Play Sound</button>
            `;

            resultsDiv.innerHTML += resultHtml;

            if (!window.audioBuffers) {
                window.audioBuffers = {};
            }
            window.audioBuffers[inputId] = audioBuffer;
        } catch (error) {
            resultsDiv.innerHTML += `<h2>${inputId}</h2><p>Error: ${error.message}</p>`;
        }
    }
}

// Add this function to the global scope so it can be called from the HTML
window.playAudio = function (inputId) {
    const audioBuffer = window.audioBuffers[inputId];
    if (audioBuffer) {
        playAudio(audioBuffer);
    } else {
        console.error(`No audio buffer found for ${inputId}`);
    }
};

async function main() {
    await init();
    audioContext = new (window.AudioContext || window.webkitAudioContext)();
    document.getElementById('decodeButton').addEventListener('click', decodeAllFiles);
}

main();