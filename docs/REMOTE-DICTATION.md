# Dictate on one computer, type on another

Requires Lectus 0.7.1 or newer on both computers, with normal membership and OS permissions.
No audio driver, streaming app or separate purchase is required for this feature.

1. **Mac:** open Settings > General > Dictation destination, choose **Receive from another computer**, then **Save destination** and **Copy connection code**.
2. **Windows:** in the same panel choose **Send to another computer**, paste the Mac code, and press **Save destination**, then **Test connection**. Keep the Alias selected as the Windows microphone.
3. Focus a text field on the Mac. Use the normal Windows dictation key and speak. Windows transcribes; Mac Lectus receives the finished text and pastes it locally.

Keep both apps running on the same local IPv4 network. Allow local-network access if asked;
the Mac also needs Accessibility permission. Network discovery follows the paired device
when its address changes. Input Leap continues to carry your keyboard/mouse; this feature
does not change its server address configuration.

The destination stays selected until you choose **This computer** and save. Receive mode
disables local recording so the shared shortcut does not also start the Mac microphone.
VB-Cable is not used by this route. Text delivery is encrypted using the connection code;
keep that code private. Audio follows your existing Windows transcription setting (local
unless you enable a cloud backend).

## If delivery fails

- Run **Test connection** in Windows. It checks the paired Mac and its permissions without typing anything.
- If the Mac cannot be found, check that both computers share the same network and that firewall/network permissions permit Lectus. Guest Wi-Fi isolation and blocked multicast can prevent discovery.
- The Windows dictation history is saved before sending. Open **Dictations** to recover a phrase. A missing acknowledgement is never retried automatically, because the Mac might already have pasted it.
- A successful acknowledgement means the Mac completed its paste command; it cannot prove that the focused third-party app accepted the text. Keep the intended editable field focused.

## Release and validation

Local checks: frontend production build, 156 native tests (5 environment-dependent tests
excluded), plus the explicitly executed LAN discovery/delivery test. The transport is also
tested on Windows and macOS by release CI. Actual Windows-to-Mac insertion, OS permission
prompts and in-app update installation still require the two installed apps.

The release workflow creates a draft in `FCisco95/lectus-releases`. Existing 0.6.0 installs
use `FCisco95/lectus`, so the same `latest.json` must also be attached to the matching release
there. Publish the main release first, then the bridge; otherwise old clients see URLs to
unpublished assets. Keep both drafts private until the packages have been reviewed.
