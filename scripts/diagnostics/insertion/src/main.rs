//! Only targets a separately owned WinForms fixture. No clipboard fallback.
#[path = "../../../../src-tauri/src/injection/sendinput.rs"]
mod sendinput;

use anyhow::{bail, ensure, Result};
use windows::Win32::{
    Foundation::HWND,
    UI::{Input::KeyboardAndMouse::{GetAsyncKeyState, GetLastInputInfo, LASTINPUTINFO},
        WindowsAndMessaging::{GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO, IsChild}},
};

fn no_modifiers() -> bool {
    // Shift, Ctrl, Alt, left/right Windows, and mouse buttons. Do not release
    // or alter a key that the user is holding; abort instead.
    [0x10, 0x11, 0x12, 0x5b, 0x5c, 0x01, 0x02].iter()
        .all(|key| unsafe { GetAsyncKeyState(*key) } >= 0)
}

fn validate_target(form: HWND, field: HWND, expected_pid: u32) -> Result<()> {
    unsafe {
        ensure!(GetForegroundWindow() == form, "Focus changed; no input sent");
        let mut pid = 0;
        let thread = GetWindowThreadProcessId(form, Some(&mut pid));
        ensure!(thread != 0 && pid == expected_pid, "Foreground owner mismatch; no input sent");
        GetWindowThreadProcessId(field, Some(&mut pid));
        ensure!(pid == expected_pid && IsChild(form, field).as_bool(), "Field owner mismatch; no input sent");
        let mut info = GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
        GetGUIThreadInfo(thread, &mut info)?;
        ensure!(info.hwndFocus == field, "Owned text field does not have focus; no input sent");
    }
    ensure!(no_modifiers(), "A modifier or mouse button is held; no input sent");
    Ok(())
}

fn last_input() -> Result<u32> {
    let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, ..Default::default() };
    ensure!(unsafe { GetLastInputInfo(&mut info) }.as_bool(), "Cannot establish input inactivity");
    Ok(info.dwTime)
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 { bail!("Only run through the owned WinForms fixture run.ps1"); }
    let form = HWND(args[0].parse::<usize>()? as *mut _);
    let field = HWND(args[1].parse::<usize>()? as *mut _);
    let expected_pid = args[2].parse::<u32>()?;
    validate_target(form, field, expected_pid)?;
    let before = last_input()?;
    std::thread::sleep(std::time::Duration::from_millis(150));
    ensure!(last_input()? == before, "User input detected; no synthetic input sent");
    validate_target(form, field, expected_pid)?;
    // One production batched SendInput call. No key chord, newline, clipboard,
    // target activation, or fallback to another insertion mechanism.
    sendinput::inject_via_sendinput("Lectus: olá, English, 汉字.")?;
    ensure!(unsafe { GetForegroundWindow() } == form, "Focus changed during input; fixture must be reviewed");
    println!("production_sendinput_completed=true target_pid_verified=true target_hwnd_verified=true field_focus_verified=true clipboard_used=false");
    Ok(())
}
