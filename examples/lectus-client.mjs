/** Browser client for Lectus's upload-only local API. No recording or token storage. */
export async function toMono16kWav(blob) {
  const decoder = new AudioContext();
  let decoded;
  try { decoded = await decoder.decodeAudioData(await blob.arrayBuffer()); }
  finally { await decoder.close(); }
  if (!Number.isFinite(decoded.duration) || decoded.duration <= 0 || decoded.duration > 120) {
    throw new Error('Choose non-empty audio no longer than 120 seconds.');
  }
  const renderer = new OfflineAudioContext(1, Math.ceil(decoded.duration * 16000), 16000);
  const source = renderer.createBufferSource();
  source.buffer = decoded;
  source.connect(renderer.destination);
  source.start();
  const pcm = (await renderer.startRendering()).getChannelData(0);
  const bytes = new ArrayBuffer(44 + pcm.length * 2);
  const view = new DataView(bytes);
  const tag = (offset, text) => [...text].forEach((char, index) => view.setUint8(offset + index, char.charCodeAt(0)));
  tag(0, 'RIFF'); view.setUint32(4, bytes.byteLength - 8, true); tag(8, 'WAVE');
  tag(12, 'fmt '); view.setUint32(16, 16, true); view.setUint16(20, 1, true);
  view.setUint16(22, 1, true); view.setUint32(24, 16000, true); view.setUint32(28, 32000, true);
  view.setUint16(32, 2, true); view.setUint16(34, 16, true); tag(36, 'data');
  view.setUint32(40, pcm.length * 2, true);
  pcm.forEach((sample, index) => view.setInt16(44 + index * 2, Math.round(Math.max(-1, Math.min(1, sample)) * 32767), true));
  return new Blob([bytes], { type: 'audio/wav' });
}

export async function transcribeWithLectus(blob, { token, port = 8756, language = 'auto', cleanup = false, signal } = {}) {
  if (!token) throw new Error('Copy the API token from Lectus Settings → API.');
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error('Invalid Lectus port.');
  const wav = await toMono16kWav(blob);
  let response;
  try {
    response = await fetch(`http://127.0.0.1:${port}/v1/transcribe`, {
      method: 'POST', credentials: 'omit', redirect: 'error', signal,
      headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'audio/wav',
        'X-Lectus-Language': language, 'X-Lectus-Cleanup': cleanup ? '1' : '0' },
      body: wav,
    });
  } catch (error) {
    if (signal?.aborted) throw error;
    throw new Error('Cannot reach Lectus. Check that its API is running, this site is allowlisted, and the browser permits access to this computer.', { cause: error });
  }
  const result = await response.json();
  if (!response.ok) throw new Error(result.error?.message || `Lectus returned HTTP ${response.status}`);
  return result;
}
