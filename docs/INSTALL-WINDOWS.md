# Install Lectus on Windows

Needs Windows 10 or 11 (64-bit), a microphone, and a graphics driver with Vulkan support.
Current NVIDIA, AMD and Intel drivers include Vulkan. No administrator rights are needed:
Lectus installs for your Windows account only, in `%LOCALAPPDATA%\Lectus`.

## Open Lectus in 3 steps

1. **Download** `Lectus_<version>_x64-setup.exe`.
2. **Open it.** Windows may show **Windows protected your PC**, because Lectus is new and is
   not code-signed yet. Select **More info**, check that the app is the Lectus setup file you
   downloaded, then select **Run anyway**.
3. **Wait a few seconds.** A small "Getting Lectus ready" window installs it, then Lectus opens
   and walks you through setup: terms, microphone, your words and your shortcut.

## If something stops you

- **"Lectus needs a graphics driver with Vulkan support":** install the latest graphics driver
  from your PC maker, or from NVIDIA, AMD or Intel, then open the setup file again. Virtual
  machines and PCs with only the basic Microsoft display driver cannot run Lectus yet.
- **Smart App Control blocked the setup file:** this Windows 11 setting blocks every app that
  is not code-signed, and it offers no "Run anyway". Lectus cannot be installed while it is
  on. Turning it off is your own security choice; on some Windows versions it cannot be
  turned back on without resetting Windows.
- **"Lectus is open. Close it and continue installing?":** choose OK to close the running
  copy and install, or Cancel to stop without changing anything.
- **A newer version is already installed:** setup asks before it replaces it with an older one.
  The default answer keeps the newer version.

## Updates

Lectus checks for updates and installs them itself, in the same small window ("Updating
Lectus"), then reopens. Settings, dictation history, shortcuts and start-with-Windows are
kept. You can also download a newer setup file and open it over the installed copy.

## Uninstall

Open **Settings > Apps > Installed apps**, find **Lectus**, and choose **Uninstall**. The
uninstaller asks whether to delete your app data (settings and dictation history) too.

## For administrators

- `Lectus_<version>_x64-setup.exe /S` installs silently for the current user.
- `/NOLAUNCH` installs without opening Lectus afterwards.
- `%LOCALAPPDATA%\Lectus\uninstall.exe /S` removes the app silently and keeps app data.
