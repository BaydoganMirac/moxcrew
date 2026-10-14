//! One locked writer owns the canonical append-only log. Projections are
//! disposable caches rebuilt on open/recovery and durable append. Reads clone
//! validated in-memory state instead of replaying and fsyncing the whole log.
use crate::domain::{project_events, validate_event};
use crate::{Error, Result};
use fs2::FileExt;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

const PROJECTIONS: &[(&str, &str)] = &[
    ("observation", "observation.json"),
    ("work", "work-items.json"),
    ("workspaces", "workspaces.json"),
    ("packs", "agent-packs.json"),
    ("teams", "agent-teams.json"),
];

pub struct Store {
    directory: PathBuf,
    // Keep the exclusive OS lock alive for the entire host lifetime. The
    // in-process mutex additionally serializes transaction/read/write paths.
    _writer_lock: File,
    lock_fingerprint: Fingerprint,
    writer: Mutex<State>,
}
struct State {
    events: Vec<Value>,
    projected: Value,
    valid_bytes: u64,
    incomplete_tail: bool,
    fingerprint: Option<Fingerprint>,
    cache_dirty: bool,
    poisoned: bool,
}
#[derive(PartialEq)]
struct Fingerprint {
    len: u64,
    modified: SystemTime,
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
    #[cfg(windows)]
    created: u64,
}
fn linked(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}
fn checked_metadata(path: &Path, directory: bool) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if linked(&metadata) {
                return Err(Error("PRIVATE_STORAGE_LINK_NOT_ALLOWED".into()));
            }
            if if directory {
                !metadata.is_dir()
            } else {
                !metadata.is_file()
            } {
                return Err(Error("PRIVATE_STORAGE_TYPE_INVALID".into()));
            }
            #[cfg(unix)]
            if !directory {
                use std::os::unix::fs::MetadataExt;
                if metadata.nlink() != 1 {
                    return Err(Error("PRIVATE_STORAGE_HARD_LINK_NOT_ALLOWED".into()));
                }
            }
            Ok(Some(metadata))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
fn existing_private_file(path: &Path) -> Result<Option<fs::Metadata>> {
    let metadata = checked_metadata(path, false)?;
    #[cfg(windows)]
    if metadata.is_some() {
        // Stable std metadata does not expose the Windows hard-link count.
        // Open the reparse point itself and query the handle before any ACL.
        let file = private_options().read(true).open(path)?;
        checked_handle(&file)?;
    }
    Ok(metadata)
}
fn file_fingerprint(path: &Path) -> Result<Option<Fingerprint>> {
    existing_private_file(path)?
        .map(|metadata| fingerprint(&metadata))
        .transpose()
}
fn fingerprint(metadata: &fs::Metadata) -> Result<Fingerprint> {
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    #[cfg(windows)]
    use std::os::windows::fs::MetadataExt;
    Ok(Fingerprint {
        len: metadata.len(),
        modified: metadata.modified()?,
        #[cfg(unix)]
        identity: (
            metadata.dev(),
            metadata.ino(),
            metadata.ctime(),
            metadata.ctime_nsec(),
        ),
        #[cfg(windows)]
        created: metadata.creation_time(),
    })
}
fn checked_handle(file: &File) -> Result<()> {
    let metadata = file.metadata()?;
    if linked(&metadata) || !metadata.is_file() {
        return Err(Error("PRIVATE_STORAGE_TYPE_INVALID".into()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(Error("PRIVATE_STORAGE_HARD_LINK_NOT_ALLOWED".into()));
        }
    }
    #[cfg(windows)]
    {
        use std::{mem::MaybeUninit, os::windows::io::AsRawHandle};
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        };
        let mut information = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), information.as_mut_ptr()) }
            == 0
        {
            return Err(Error("PRIVATE_STORAGE_LINK_CHECK_FAILED".into()));
        }
        if unsafe { information.assume_init() }.nNumberOfLinks != 1 {
            return Err(Error("PRIVATE_STORAGE_HARD_LINK_NOT_ALLOWED".into()));
        }
    }
    Ok(())
}

pub fn private_directory(directory: &Path) -> Result<()> {
    checked_metadata(directory, true)?;
    fs::create_dir_all(directory)?;
    checked_metadata(directory, true)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let directory_handle = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
            .open(directory)?;
        directory_handle.set_permissions(fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(windows)]
    windows_private_acl(directory, true)?;
    Ok(())
}
/// Restrict an existing private state file before consuming it. Missing files
/// are harmless; their creation inherits the already protected data directory.
pub fn secure_file(path: &Path) -> Result<()> {
    match checked_metadata(path, false)? {
        Some(_) => {
            let file = private_options().read(true).open(path)?;
            checked_handle(&file)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(0o600))?;
            }
            #[cfg(windows)]
            windows_private_acl(path, false)?;
            Ok(())
        }
        None => Ok(()),
    }
}
/// Return a checked private reader so callers can apply their own byte limits.
/// Never follow a symbolic link, reparse point or multiply linked inode.
pub fn open_private_reader(path: &Path) -> Result<File> {
    secure_file(path)?;
    let file = private_options().read(true).open(path)?;
    checked_handle(&file)?;
    Ok(file)
}
/// Read a regular private file without following a symbolic link/reparse point.
pub fn read_private_file(path: &Path) -> Result<Vec<u8>> {
    let mut file = open_private_reader(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(windows)]
fn windows_private_acl(path: &Path, directory: bool) -> Result<()> {
    use std::{
        ffi::c_void,
        os::windows::ffi::OsStrExt,
        ptr::{null, null_mut},
        sync::OnceLock,
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, LocalFree, HANDLE},
        Security::Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            SetNamedSecurityInfoW, SDDL_REVISION_1, SE_FILE_OBJECT,
        },
        Security::{
            GetSecurityDescriptorDacl, GetTokenInformation, TokenUser, DACL_SECURITY_INFORMATION,
            PROTECTED_DACL_SECURITY_INFORMATION, TOKEN_QUERY, TOKEN_USER,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };
    struct Token(HANDLE);
    impl Drop for Token {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    struct Local(*mut c_void);
    impl Drop for Local {
        fn drop(&mut self) {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
    fn permission_error() -> Error {
        Error("PRIVATE_STORAGE_PERMISSIONS_FAILED".into())
    }
    static OWNER_SID: OnceLock<String> = OnceLock::new();
    let sid = if let Some(sid) = OWNER_SID.get() {
        sid.clone()
    } else {
        // Query the process identity; an environment variable or account name
        // cannot substitute an attacker-selected SID in the security descriptor.
        let mut handle = null_mut();
        unsafe {
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) == 0 {
                return Err(permission_error());
            }
        }
        let token = Token(handle);
        let mut needed = 0u32;
        unsafe {
            GetTokenInformation(token.0, TokenUser, null_mut(), 0, &mut needed);
        }
        if (needed as usize) < std::mem::size_of::<TOKEN_USER>() || needed > 65536 {
            return Err(permission_error());
        }
        // TOKEN_USER contains pointers and needs pointer-aligned storage.
        let mut storage = vec![0u64; (needed as usize).div_ceil(8)];
        unsafe {
            if GetTokenInformation(
                token.0,
                TokenUser,
                storage.as_mut_ptr().cast(),
                needed,
                &mut needed,
            ) == 0
            {
                return Err(permission_error());
            }
        }
        let user = unsafe { &*storage.as_ptr().cast::<TOKEN_USER>() };
        let mut sid_ptr = null_mut();
        unsafe {
            if ConvertSidToStringSidW(user.User.Sid, &mut sid_ptr) == 0 {
                return Err(permission_error());
            }
        }
        let sid_allocation = Local(sid_ptr.cast());
        let mut length = 0usize;
        unsafe {
            while length < 256 && *sid_ptr.add(length) != 0 {
                length += 1;
            }
        }
        if length == 256 {
            return Err(permission_error());
        }
        let sid = String::from_utf16(unsafe { std::slice::from_raw_parts(sid_ptr, length) })
            .map_err(|_| permission_error())?;
        drop(sid_allocation);
        let _ = OWNER_SID.set(sid.clone());
        sid
    };
    // Do not propagate a new directory ACL into existing children: one may be
    // a hard link to an outside inode that must remain untouched. Every private
    // file receives its own protected ACL after its no-link handle check, and
    // the owner-only directory blocks traversal while a new empty file is set up.
    let _ = directory;
    let sddl: Vec<u16> = format!("D:P(A;;FA;;;{sid})(A;;FA;;;SY)")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = null_mut();
    unsafe {
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            null_mut(),
        ) == 0
        {
            return Err(permission_error());
        }
    }
    let descriptor_allocation = Local(descriptor);
    let mut present = 0;
    let mut defaulted = 0;
    let mut dacl = null_mut();
    unsafe {
        if GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted) == 0
            || present == 0
            || dacl.is_null()
        {
            return Err(permission_error());
        }
    }
    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // Replace (rather than merge) the DACL and block inherited grants. Existing
    // explicit Everyone/Users ACEs must not survive an owner-selected directory.
    let status = unsafe {
        SetNamedSecurityInfoW(
            name.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            dacl,
            null(),
        )
    };
    drop(descriptor_allocation);
    if status != 0 {
        return Err(permission_error());
    }
    Ok(())
}
fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x00200000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    options
}
fn private_file(file: &File) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    {
        let _ = file;
    }
    Ok(())
}
fn directory_sync(directory: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
            .open(directory)?
            .sync_all()?;
    }
    // Windows std File cannot open a directory for fsync. rename() uses
    // MoveFileExW(REPLACE_EXISTING), keeping the old target until replacement.
    #[cfg(not(unix))]
    {
        let _ = directory;
    }
    Ok(())
}
/// Write and fsync a private same-directory temporary, then atomically replace
/// the target. Never remove the old target before the successful rename.
/// Windows files receive a protected current-owner+SYSTEM DACL, while Unix
/// enforces 0600 independently of the caller's umask.
pub fn atomic_write(target: &Path, bytes: &[u8]) -> Result<()> {
    let directory = target
        .parent()
        .ok_or_else(|| Error("INVALID_STORAGE_PATH".into()))?;
    private_directory(directory)?;
    existing_private_file(target)?;
    let filename = target
        .file_name()
        .ok_or_else(|| Error("INVALID_STORAGE_PATH".into()))?
        .to_string_lossy();
    let temporary = directory.join(format!(".{filename}.{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let mut options = private_options();
        let mut file = options.write(true).create_new(true).open(&temporary)?;
        checked_handle(&file)?;
        secure_file(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, target)?;
        directory_sync(directory)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

impl Store {
    pub fn open(directory: PathBuf) -> Result<Self> {
        private_directory(&directory)?;
        let lock_path = directory.join("writer.lock");
        checked_metadata(&lock_path, false)?;
        let lock = private_options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        checked_handle(&lock)?;
        lock.try_lock_exclusive()
            .map_err(|_| Error("LOCAL_WRITER_ALREADY_RUNNING".into()))?;
        private_file(&lock)?;
        secure_file(&lock_path)?;
        let lock_fingerprint = fingerprint(&lock.metadata()?)?;
        if file_fingerprint(&lock_path)?.as_ref() != Some(&lock_fingerprint) {
            return Err(Error("LOCAL_WRITER_LOCK_CHANGED".into()));
        }
        // Replay once before returning a host. Missing/stale cache files cannot
        // hide canonical corruption, and reads never write projection files.
        let state = Self::read_log(&directory)?;
        let store = Self {
            directory,
            _writer_lock: lock,
            lock_fingerprint,
            writer: Mutex::new(state),
        };
        {
            let mut state = store.guard()?;
            store.write_projections(&state.projected)?;
            state.cache_dirty = false;
        }
        Ok(store)
    }
    fn guard(&self) -> Result<std::sync::MutexGuard<'_, State>> {
        self.writer
            .lock()
            .map_err(|_| Error("LOCAL_WRITER_POISONED".into()))
    }
    fn read_log(directory: &Path) -> Result<State> {
        let path = directory.join("events.jsonl");
        let bytes = if checked_metadata(&path, false)?.is_some() {
            read_private_file(&path)?
        } else {
            Vec::new()
        };
        let incomplete_tail = !bytes.is_empty() && bytes.last() != Some(&b'\n');
        let valid_bytes = if incomplete_tail {
            bytes
                .iter()
                .rposition(|b| *b == b'\n')
                .map(|i| i + 1)
                .unwrap_or(0)
        } else {
            bytes.len()
        };
        let complete = std::str::from_utf8(&bytes[..valid_bytes])
            .map_err(|_| Error("LOCAL_STORAGE_CORRUPT: invalid UTF-8".into()))?;
        let mut events = Vec::new();
        if !complete.is_empty() {
            for line in complete[..complete.len() - 1].split('\n') {
                if line.is_empty() {
                    return Err(Error("LOCAL_STORAGE_CORRUPT: empty complete record".into()));
                }
                let event: Value = serde_json::from_str(line).map_err(|_| {
                    Error("LOCAL_STORAGE_CORRUPT: malformed complete record".into())
                })?;
                events.push(event);
            }
        }
        let projected = project_events(&events)?;
        let fingerprint = file_fingerprint(&path)?;
        if fingerprint
            .as_ref()
            .is_some_and(|f| f.len != bytes.len() as u64)
        {
            return Err(Error("LOCAL_STORAGE_EXTERNAL_CHANGE".into()));
        }
        Ok(State {
            events,
            projected,
            valid_bytes: valid_bytes as u64,
            incomplete_tail,
            fingerprint,
            cache_dirty: true,
            poisoned: false,
        })
    }
    fn ensure_unchanged(&self, state: &State) -> Result<()> {
        if state.poisoned {
            return Err(Error("LOCAL_WRITER_RECOVERY_REQUIRED".into()));
        }
        self.ensure_lock()?;
        if file_fingerprint(&self.directory.join("events.jsonl"))? != state.fingerprint {
            return Err(Error("LOCAL_STORAGE_EXTERNAL_CHANGE".into()));
        }
        Ok(())
    }
    fn ensure_lock(&self) -> Result<()> {
        if checked_metadata(&self.directory, true)?.is_none()
            || file_fingerprint(&self.directory.join("writer.lock"))?.as_ref()
                != Some(&self.lock_fingerprint)
        {
            return Err(Error("LOCAL_WRITER_LOCK_CHANGED".into()));
        }
        Ok(())
    }
    pub fn read_events(&self) -> Result<Vec<Value>> {
        let state = self.guard()?;
        self.ensure_unchanged(&state)?;
        Ok(state.events.clone())
    }
    pub fn projections(&self) -> Result<Value> {
        let state = self.guard()?;
        self.ensure_unchanged(&state)?;
        Ok(state.projected.clone())
    }
    pub fn append(&self, additions: Vec<Value>) -> Result<()> {
        let mut state = self.guard()?;
        self.ensure_unchanged(&state)?;
        self.append_locked(&mut state, additions)
    }
    pub fn transaction<F>(&self, build: F) -> Result<Value>
    where
        F: FnOnce(&[Value]) -> Result<(Vec<Value>, Value)>,
    {
        let mut state = self.guard()?;
        self.ensure_unchanged(&state)?;
        let (additions, result) = build(&state.events)?;
        self.append_locked(&mut state, additions)?;
        Ok(result)
    }
    fn append_locked(&self, state: &mut State, additions: Vec<Value>) -> Result<()> {
        if additions.is_empty() && !state.incomplete_tail {
            if state.cache_dirty {
                self.write_projections(&state.projected)?;
                state.cache_dirty = false;
            }
            return Ok(());
        }
        let mut combined = state.events.clone();
        for event in &additions {
            validate_event(event)?;
        }
        combined.extend(additions.iter().cloned());
        let projected = project_events(&combined)?;
        let mut bytes = Vec::new();
        for event in additions {
            serde_json::to_writer(&mut bytes, &event)?;
            bytes.push(b'\n');
        }
        let path = self.directory.join("events.jsonl");
        let durable = (|| -> Result<()> {
            existing_private_file(&path)?;
            let mut file = private_options()
                .read(true)
                .append(true)
                .create(true)
                .open(&path)?;
            checked_handle(&file)?;
            private_file(&file)?;
            secure_file(&path)?;
            if state.incomplete_tail {
                file.set_len(state.valid_bytes)?;
                file.sync_all()?;
            }
            if !bytes.is_empty() {
                file.write_all(&bytes)?;
                file.sync_all()?;
            }
            drop(file);
            directory_sync(&self.directory)?;
            Ok(())
        })();
        if let Err(error) = durable {
            // A failed write/fsync may have changed some complete records. Do
            // not serve a stale cache or retry a side effect before recovery.
            state.poisoned = true;
            return Err(error);
        }
        // The log is canonical and already durable. Update memory before any
        // disposable projection write; a cache failure cannot hide a receipt.
        state.events = combined;
        state.projected = projected;
        state.valid_bytes += bytes.len() as u64;
        state.incomplete_tail = false;
        state.fingerprint = file_fingerprint(&path)?;
        state.cache_dirty = true;
        self.write_projections(&state.projected)?;
        state.cache_dirty = false;
        Ok(())
    }
    fn write_projections(&self, projected: &Value) -> Result<()> {
        for (key, filename) in PROJECTIONS {
            let path = self.directory.join(filename);
            existing_private_file(&path)?;
            if projected[key].is_null() {
                match fs::remove_file(&path) {
                    Ok(()) => directory_sync(&self.directory)?,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.into()),
                }
            } else {
                let mut bytes = serde_json::to_vec(&projected[key])?;
                bytes.push(b'\n');
                atomic_write(&path, &bytes)?;
            }
        }
        Ok(())
    }
    /// Explicit offline recovery validates canonical bytes and repairs caches.
    /// It also clears a failed-write poison; ordinary reads never do disk repair.
    pub fn recover(&self) -> Result<Value> {
        let mut state = self.guard()?;
        self.ensure_lock()?;
        *state = Self::read_log(&self.directory)?;
        self.write_projections(&state.projected)?;
        state.cache_dirty = false;
        Ok(state.projected.clone())
    }
}
