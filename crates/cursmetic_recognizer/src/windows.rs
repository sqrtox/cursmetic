use std::path::PathBuf;

use windows::Win32::Foundation::{GetLastError, HMODULE};
use windows::Win32::System::LibraryLoader::{LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::Win32::UI::WindowsAndMessaging::LoadStringW;
use windows::core::{PCWSTR, PWSTR};

use crate::error::{Error, Result};

pub struct ResourceId(pub Option<u32>);

pub fn load_string(module: HMODULE, id: &ResourceId) -> Option<String> {
    let id = id.0?;

    let mut buffer = [0u16; 256];
    let len = unsafe {
        LoadStringW(
            Some(module.into()),
            id,
            PWSTR(buffer.as_mut_ptr()),
            buffer.len() as _,
        )
    };

    if len == 0 {
        return None;
    }

    Some(String::from_utf16_lossy(&buffer[..len as usize]))
}

fn system_direcotry() -> Result<PathBuf> {
    let mut buffer = [0u16; 260];
    // TODO: バッファを上回る場合がある
    let len = unsafe { GetSystemDirectoryW(Some(&mut buffer)) };

    if len == 0 {
        return Err(Error::from(unsafe { GetLastError() }));
    }

    Ok(PathBuf::from(String::from_utf16_lossy(
        &buffer[..len as usize],
    )))
}

fn main_cpl_path() -> Result<PathBuf> {
    let path = system_direcotry()?.join("main.cpl");

    if !path.exists() {
        return Err(Error::PathNotExists(path));
    }

    Ok(path)
}

pub fn main_cpl() -> Result<HMODULE> {
    let main_cpl_path = main_cpl_path()?;
    let main_cpl_wide: Vec<u16> = main_cpl_path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    Ok(unsafe {
        LoadLibraryExW(
            PCWSTR(main_cpl_wide.as_ptr()),
            None,
            LOAD_LIBRARY_AS_DATAFILE,
        )?
    })
}
