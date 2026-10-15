//! Narrow Windows DACL adapter. No networking, input or ViGEm types.
use std::{io, os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::{
        Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW,
        DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, SetFileSecurityW,
    },
};

pub fn protect_directory(path: &Path) -> io::Result<()> {
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // Protected DACL, inherited owner rights only. Administrators can still take ownership.
    let sddl: Vec<u16> = "D:P(A;OICI;FA;;;OW)"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = std::ptr::null_mut();
    // SAFETY: Both inputs are owned NUL terminated UTF-16 buffers. The API allocates
    // descriptor, used only until LocalFree below on both success and failure.
    unsafe {
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        let success = SetFileSecurityW(
            path.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        );
        let error = if success == 0 {
            Some(io::Error::last_os_error())
        } else {
            None
        };
        LocalFree(descriptor);
        if let Some(error) = error {
            return Err(error);
        }
    }
    Ok(())
}
