//! Same-user pipe ownership and peer checks. No handle or SID escapes its owner.
use std::{ffi::c_void, io};

use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree},
        Security::{
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
                GetSecurityInfo, SE_KERNEL_OBJECT,
            },
            CopySid, EqualSid, GetLengthSid, GetTokenInformation, OWNER_SECURITY_INFORMATION,
            PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
        },
        System::Threading::{
            GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
        },
    },
    core::{PCWSTR, PWSTR},
};

struct OwnedHandle(HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: this owner holds one real process/token handle, never a pseudo handle.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

struct LocalMemory(*mut c_void);
impl Drop for LocalMemory {
    fn drop(&mut self) {
        // SAFETY: only Windows LocalAlloc-family output is retained here.
        let _ = unsafe { LocalFree(Some(HLOCAL(self.0))) };
    }
}

pub(super) struct User(Vec<u32>);

impl User {
    pub(super) fn current() -> io::Result<Self> {
        // SAFETY: GetCurrentProcess has no preconditions and returns a pseudo handle.
        Self::of_process(unsafe { GetCurrentProcess() })
    }

    fn of_process(process: HANDLE) -> io::Result<Self> {
        let mut token = HANDLE::default();
        // SAFETY: the supplied live process handle permits token inspection.
        unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }.map_err(io::Error::other)?;
        let token = OwnedHandle(token);
        let mut size = 0;
        // SAFETY: a null output with zero length obtains the required byte count.
        let _ = unsafe { GetTokenInformation(token.0, TokenUser, None, 0, &mut size) };
        if size == 0 || size > 64 * 1024 {
            return Err(io::Error::other("Could not read the IPC user identity"));
        }
        let mut storage = vec![0_usize; (size as usize).div_ceil(size_of::<usize>())];
        // SAFETY: the aligned allocation has at least the requested byte length.
        unsafe {
            GetTokenInformation(
                token.0,
                TokenUser,
                Some(storage.as_mut_ptr().cast()),
                size,
                &mut size,
            )
        }
        .map_err(io::Error::other)?;
        // SAFETY: successful TokenUser output starts with a TOKEN_USER structure.
        let source = unsafe { &*storage.as_ptr().cast::<TOKEN_USER>() }.User.Sid;
        // SAFETY: the SID remains valid within the retained token output.
        let size = unsafe { GetLengthSid(source) };
        if size == 0 || size > 68 {
            return Err(io::Error::other("Invalid IPC user identity"));
        }
        let mut sid = Self(vec![0; (size as usize).div_ceil(size_of::<u32>())]);
        // SAFETY: the DWORD-aligned destination has enough space for this SID.
        unsafe { CopySid(size, sid.raw(), source) }.map_err(io::Error::other)?;
        Ok(sid)
    }

    fn raw(&mut self) -> PSID {
        PSID(self.0.as_mut_ptr().cast())
    }

    pub(super) fn name(&mut self) -> io::Result<String> {
        let mut text = PWSTR::null();
        // SAFETY: this user owns a valid SID; Windows allocates the output string.
        unsafe { ConvertSidToStringSidW(self.raw(), &mut text) }.map_err(io::Error::other)?;
        let _memory = LocalMemory(text.0.cast());
        // SAFETY: a successful conversion returns a NUL-terminated UTF-16 string.
        unsafe { text.to_string() }.map_err(io::Error::other)
    }

    pub(super) fn check_process(&mut self, process_id: u32) -> io::Result<()> {
        // SAFETY: limited query access inspects identity without changing the peer.
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }
            .map_err(io::Error::other)?;
        let process = OwnedHandle(process);
        let mut peer = Self::of_process(process.0)?;
        // SAFETY: both SID allocations remain valid during comparison.
        if unsafe { EqualSid(self.raw(), peer.raw()) }.is_err() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "The IPC peer belongs to another user",
            ));
        }
        Ok(())
    }

    pub(super) fn check_pipe_owner(&mut self, pipe: HANDLE) -> io::Result<()> {
        let mut owner = PSID::default();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        // SAFETY: the connected pipe handle grants READ_CONTROL via its user-only ACL.
        unsafe {
            GetSecurityInfo(
                pipe,
                SE_KERNEL_OBJECT,
                OWNER_SECURITY_INFORMATION,
                Some(&mut owner),
                None,
                None,
                None,
                Some(&mut descriptor),
            )
        }
        .ok()
        .map_err(io::Error::other)?;
        let _memory = LocalMemory(descriptor.0);
        // SAFETY: the returned owner is retained by this security descriptor.
        if unsafe { EqualSid(self.raw(), owner) }.is_err() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "The IPC pipe belongs to another user",
            ));
        }
        Ok(())
    }
}

pub(super) struct Descriptor(LocalMemory);
impl Descriptor {
    pub(super) fn for_user(user: &mut User) -> io::Result<Self> {
        let name = user.name()?;
        let value: Vec<u16> = format!("O:{name}D:P(A;;GA;;;{name})")
            .encode_utf16()
            .chain([0])
            .collect();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        // SAFETY: the input is NUL terminated; Windows allocates the descriptor.
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(value.as_ptr()),
                1,
                &mut descriptor,
                None,
            )
        }
        .map_err(io::Error::other)?;
        Ok(Self(LocalMemory(descriptor.0)))
    }

    pub(super) fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.0.0,
            bInheritHandle: false.into(),
        }
    }
}
