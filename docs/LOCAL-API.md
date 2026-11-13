# Connect another app to Lectus

The local API transcribes audio a caller already has. It never starts the microphone, pastes text, writes dictation history, launches Claude or sends audio to a cloud provider. A local model and valid Lectus membership are required, even when the ordinary dictation settings use cloud transcription.

## Set up

1. In Settings → API, choose a free port (default 8756), enable the API and save. It binds only to `127.0.0.1`.
2. For a browser client, add its exact origin, such as `http://localhost:3000`, and save. Scheme and port matter. Wildcards, paths and trailing slashes are rejected. Leave the list empty for command-line clients only.
3. Reveal or copy the token into the client. Keep it out of source control, URLs and logs. Regeneration invalidates the previous token.
4. Send a mono 16 kHz WAV, no longer than 120 seconds or larger than 8 MiB. The API returns text; your client decides where to display it.

The API starts off. Its enabled state is visible in the pill and tray tooltip. Settings shows only the five most recent calls' metadata, never transcript contents. API configuration and token are stored together in `api.json` in Lectus's app-data folder, separately from general settings.

## Requests

All health/transcription requests require `Authorization: Bearer <token>`. Browser CORS preflight is the token-free exception and succeeds only for an allowlisted origin and supported method/headers.

`GET http://127.0.0.1:8756/v1/health` checks membership, availability and busy state. `POST /v1/transcribe` accepts a raw WAV body with `Content-Type: audio/wav`.

Optional headers:

- `X-Lectus-Language: pt` — supported Whisper language code or `auto`; omitted uses the saved language.
- `X-Lectus-Cleanup: 1` — applies cleanup only when the user's local cleanup engine is enabled and downloaded. Cloud and CLI cleanup are never invoked by this API.

Successful transcription returns `text`, `language`, `duration_ms` (audio length), `transcribe_ms` (processing time) and `engine: "local"`. Errors return `{ "error": { "code": "...", "message": "..." } }` with the corresponding HTTP error status. The API rejects simultaneous transcription jobs and requests while dictation/model loading is active. The limit is 30 accepted authenticated requests per minute; rejected requests must not be retried in a tight loop.

PowerShell, using a token already supplied through your environment:

```powershell
curl.exe http://127.0.0.1:8756/v1/health -H "Authorization: Bearer $env:LECTUS_TOKEN"
curl.exe http://127.0.0.1:8756/v1/transcribe -H "Authorization: Bearer $env:LECTUS_TOKEN" -H "Content-Type: audio/wav" --data-binary "@clip.wav"
```

## Browser/Organic integration

Use [examples/lectus-client.mjs](../examples/lectus-client.mjs). It converts an existing audio Blob to mono 16 kHz PCM WAV, sends the request and surfaces errors. It does not record audio or persist the token. A browser app must obtain recording permission itself if it chooses to use `getUserMedia`.

```js
import { transcribeWithLectus } from './lectus-client.mjs';
const result = await transcribeWithLectus(audioBlob, {
  token: tokenFromUserSettings,
  port: 8756,
  language: 'pt',
});
transcriptField.value = result.text;
```

Browser permission is separate from Lectus's allowlist. Chrome/Edge may ask for access to this computer's loopback/local network; blocked browser permission prevents the request even with the correct token and origin. Handle that failure in the client and direct the user to the site's permission settings. Do not disable browser protections to connect. See [Chrome's local-network guidance](https://developer.chrome.com/blog/local-network-access).

The example is ready for integration into Organic but no Organic repository or production origin has been changed here. A real browser-origin connection remains a distinct integration test from backend socket tests and mocked UI checks.

## Troubleshooting

- Connection refused: save enabled settings and check the displayed port/running state. A port conflict stays visible; Lectus does not silently choose another port.
- `unauthorized` / `forbidden_origin`: refresh the client token or exact origin. No wildcard access is supported.
- `locked` / `engine_unavailable`: resolve membership or download/select a local model in Lectus.
- `busy` / `rate_limited`: wait for current dictation/inference, then retry deliberately.
- Invalid audio: convert to the required WAV format; webm/opus from MediaRecorder cannot be uploaded directly.

Disabling closes the listener and rejects uploads that have not yet begun inference. An inference already executing cannot be interrupted mid-model call; it is bounded by the input-duration limit and cannot capture or insert anything.
