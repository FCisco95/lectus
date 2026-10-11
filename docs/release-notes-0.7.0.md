# Lectus 0.7.0

Dictate with the microphone on your Windows PC and insert the finished text on a paired Mac.

- In Settings > General, choose Receive on the Mac and copy its connection code to Send on Windows.
- The computers find each other on the local network, including after an IP address changes.
- Text delivery is encrypted. Receive mode disables the local microphone; failed delivery is shown and the Windows dictation history remains available.
- No virtual audio cable or paid audio receiver is needed. Local dictation remains the default.
- Includes the existing app-shell interface and membership improvements from the development branch, preserving that interface for users already running it.

Both computers need this version. Allow Lectus on the local network and grant Accessibility on the Mac if prompted. Keep the target text field focused.

## Draft validation

Windows frontend/native checks and a real local-network discovery/delivery test passed.
Release CI also runs the transport tests on Windows and macOS. Actual cross-computer paste,
the Mac permission prompts and installation through the old updater endpoint remain to be
checked on the user's computers. This draft is not an end-to-end verification claim.
