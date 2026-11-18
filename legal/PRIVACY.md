# Lectus Privacy Notice: factual draft

Controller: João Francisco Vieira, an individual based in Portugal, trading as Bakehouse Labs.

Privacy contact: bakehouselabs@proton.me.

This notice describes the implementation being prepared. The controller's geographic address, controller/processor roles, lawful bases, international-transfer disclosures and applicable rights/contact procedures need review before distribution. It is not a completed GDPR compliance statement.

## Microphone and local processing

First-run setup must be completed before dictation and the persistent microphone buffer start. After setup, except in Receive mode, Lectus keeps approximately half a second of audio in a rolling memory buffer to avoid clipping the first word. During dictation it collects the spoken audio for recognition. Normal dictation does not save a recording file. Local recognition and local text processing run on the device.

During first-run setup the microphone opens only when you start the microphone test or a word check, and that audio is kept in memory and discarded after it is transcribed.

## Vocabulary folder scan

If you choose a folder during setup, Lectus reads its text files on your device to suggest vocabulary. Only the words you select are saved. File contents are not stored and do not leave your device.

## Optional external processing

When you enable a configured speech provider, captured audio is uploaded to its audio endpoint. Configured cloud cleanup or translation sends transcript text to its text endpoint, even if recognition was local. You must explicitly allow processing by that configured provider. Changing the provider requires a new choice. Provider retention, training, region and account terms must be checked with the provider; Lectus cannot promise zero retention by third parties.

Claude CLI cleanup is a separate optional path: it sends the transcript through the installed, signed-in Claude command-line application and its configured service. Its own data handling and account terms apply.

## History, settings and clipboard

Up to 500 dictation entries are stored locally in history.json until cleared or evicted. Translated entries may include original text and language to support recovery. History and provider credentials in local configuration are not encrypted by Lectus; protect the operating-system account and device. The app uses OS clipboard/typing facilities to insert text. Clipboard restoration is best effort and may not restore every original clipboard format.

## Paired computers and local API

Send mode transmits final text over an authenticated, encrypted connection to your paired computer. The receiving computer inserts it locally. Pairing/discovery uses the local network. The optional loopback API accepts uploaded audio only; it does not start the microphone, insert text, write dictation history or invoke cloud services. Its last-five-call list contains metadata, not transcript content. API credentials are stored locally in api.json.

## Network services beyond dictation

Membership checks use wallet-related information and external Solana RPC/token-price services. Updates contact GitHub; model downloads contact model hosting services such as Hugging Face. Those services receive normal network metadata. These activities mean local inference is not a promise that the whole application never makes a network request. No general advertising/usage analytics system was identified in the inspected code; this is not a promise about third-party operating-system or provider telemetry.

## Your choices

Choose local processing to avoid sending dictation audio/text to a model provider. Disable optional cloud features or revoke their processing allowance; existing provider-side data remains subject to that provider's policies. Clear local history in the app, remove credentials you no longer need, disable the local API or regenerate its token, and unlink a wallet when appropriate. Closing the application stops its microphone stream.

## Legal acknowledgements

When final Terms are available, their accepted version/content fingerprint and acceptance time are stored on this device. This is a local receipt, not tamper-proof independent evidence. Acknowledging the Privacy Notice is not blanket consent to optional processing. Before public distribution the final notice must add the controller's address and explain applicable data rights, including the right to complain to the Portuguese data protection authority (CNPD), with a working contact route.

The optional Gemma model agreement has its own local receipt in model-license-gemma-3.json, containing only the model/document version, content fingerprint and acceptance time. Refusing it or revoking it blocks future Gemma use without deleting previous dictations or model files. An already-admitted processing operation may finish. The receipt is not uploaded by Lectus and does not grant cloud-processing permission.
