//! Handle-relative Windows I/O for job outputs and owner-selected exports.
use super::{invalid, Result};
use std::{
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions},
    io::{self, Write},
    mem::{offset_of, size_of},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::OpenOptionsExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Component, Path, PathBuf, Prefix},
    ptr::{null, null_mut},
};
use windows_sys::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            FileRenameInformation, NtCreateFile, NtSetInformationFile, FILE_CREATE,
            FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_REPARSE_POINT,
            FILE_RENAME_INFORMATION, FILE_SYNCHRONOUS_IO_NONALERT,
        },
    },
    Win32::{
        Foundation::{
            RtlNtStatusToDosError, ERROR_NO_MORE_FILES, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE,
            UNICODE_STRING,
        },
        Storage::FileSystem::*,
        System::IO::IO_STATUS_BLOCK,
    },
};

const DIRECTORY_ACCESS: u32 = FILE_TRAVERSE | FILE_READ_ATTRIBUTES;
const SHARE: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE;
const INVALID_PATH: &str = "PROJECT_RESULT_PATH_INVALID";
const UNAVAILABLE: &str = "PROJECT_RESULT_UNAVAILABLE";

fn valid_name(name: &OsStr) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    let numbered_device = stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
        .is_some_and(|n| {
            matches!(
                n,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        });
    !name.is_empty()
        && !name.ends_with(['.', ' '])
        && !name.chars().any(|c| c < ' ' || "<>:\"/\\|?*".contains(c))
        && !matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        )
        && !numbered_device
        && name.encode_utf16().count() <= 255
}

fn status_result(status: i32) -> io::Result<()> {
    if status >= 0 {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        ))
    }
}

fn information(file: &File) -> io::Result<BY_HANDLE_FILE_INFORMATION> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(info)
}

fn verify(file: &File, directory: bool) -> io::Result<()> {
    let info = information(file)?;
    if unsafe { GetFileType(file.as_raw_handle()) } != FILE_TYPE_DISK
        || info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || (info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0) != directory
        || (!directory && info.nNumberOfLinks != 1)
    {
        return Err(io::Error::from(io::ErrorKind::InvalidData));
    }
    Ok(())
}

// RootDirectory plus a single validated component prevents an ancestor swap
// from redirecting an open. Synchronous handles are required by std::fs::File.
fn child(
    parent: &File,
    name: &OsStr,
    directory: bool,
    access: u32,
    disposition: u32,
) -> io::Result<File> {
    if !valid_name(name) {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    let mut units = name.encode_wide().collect::<Vec<_>>();
    let bytes = (units.len() * 2) as u16;
    let mut name = UNICODE_STRING {
        Length: bytes,
        MaximumLength: bytes,
        Buffer: units.as_mut_ptr(),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: parent.as_raw_handle(),
        ObjectName: &mut name,
        Attributes: OBJ_CASE_INSENSITIVE | OBJ_DONT_REPARSE,
        ..Default::default()
    };
    let kind = if directory {
        FILE_DIRECTORY_FILE
    } else {
        FILE_NON_DIRECTORY_FILE
    };
    let mut handle = null_mut();
    let mut ios = IO_STATUS_BLOCK::default();
    // Existing source files may be open in an editor. A new, private staging
    // file excludes other writers, renames and newly acquired hard links.
    let share = if disposition == FILE_CREATE && !directory {
        0
    } else {
        SHARE
    };
    unsafe {
        status_result(NtCreateFile(
            &mut handle,
            access | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            &attributes,
            &mut ios,
            null(),
            FILE_ATTRIBUTE_NORMAL,
            share,
            disposition,
            kind | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            null(),
            0,
        ))?;
        let file = File::from_raw_handle(handle);
        verify(&file, directory)?;
        Ok(file)
    }
}

fn identity(file: &File) -> io::Result<(u32, u32, u32)> {
    let info = information(file)?;
    Ok((
        info.dwVolumeSerialNumber,
        info.nFileIndexHigh,
        info.nFileIndexLow,
    ))
}

fn directory(path: &Path, access: u32, excluded: Option<(u32, u32, u32)>) -> Result<File> {
    let mut components = path.components().peekable();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return Err(invalid(INVALID_PATH));
    };
    if !matches!(
        prefix.kind(),
        Prefix::Disk(_) | Prefix::VerbatimDisk(_) | Prefix::UNC(_, _) | Prefix::VerbatimUNC(_, _)
    ) || !path.is_absolute()
    {
        return Err(invalid(INVALID_PATH));
    }
    if components.peek() == Some(&Component::RootDir) {
        components.next();
    }
    let root = path
        .ancestors()
        .last()
        .ok_or_else(|| invalid(INVALID_PATH))?;
    let mut handle = OpenOptions::new()
        .access_mode(
            DIRECTORY_ACCESS
                | if components.peek().is_none() {
                    access
                } else {
                    0
                },
        )
        .share_mode(SHARE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(root)?;
    verify(&handle, true)?;
    loop {
        if excluded.is_some() && Some(identity(&handle)?) == excluded {
            return Err(invalid("PROJECT_RESULT_EXPORT_PATH_INVALID"));
        }
        match components.next() {
            None => return Ok(handle),
            Some(Component::Normal(name)) => {
                let access = DIRECTORY_ACCESS
                    | if components.peek().is_none() {
                        access
                    } else {
                        0
                    };
                handle = child(&handle, name, true, access, FILE_OPEN)?;
            }
            _ => return Err(invalid(INVALID_PATH)),
        }
    }
}

pub(crate) fn open_result(root: &Path, relative: &Path) -> Result<File> {
    let mut handle = directory(root, DIRECTORY_ACCESS, None).map_err(|_| invalid(UNAVAILABLE))?;
    let mut parts = relative.components().peekable();
    while let Some(part) = parts.next() {
        let Component::Normal(name) = part else {
            return Err(invalid(INVALID_PATH));
        };
        if !valid_name(name) {
            return Err(invalid(INVALID_PATH));
        }
        if parts.peek().is_none() {
            return child(&handle, name, false, FILE_GENERIC_READ, FILE_OPEN)
                .map_err(|_| invalid(UNAVAILABLE));
        }
        handle = child(&handle, name, true, DIRECTORY_ACCESS, FILE_OPEN)
            .map_err(|_| invalid(UNAVAILABLE))?;
    }
    Err(invalid(INVALID_PATH))
}

// Enumerate the checked directory itself. The final source file is reopened
// and checked separately before its name or contents enter the result list.
pub(super) fn entries(path: &Path, limit: usize) -> Result<Vec<(PathBuf, bool, bool)>> {
    let handle = directory(path, FILE_LIST_DIRECTORY, None)?;
    let mut items = Vec::new();
    let mut buffer = vec![0u64; 8192];
    loop {
        let ok = unsafe {
            GetFileInformationByHandleEx(
                handle.as_raw_handle(),
                FileIdBothDirectoryInfo,
                buffer.as_mut_ptr().cast(),
                size_of_val(buffer.as_slice()) as u32,
            )
        };
        if ok == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                return Ok(items);
            }
            return Err(error.into());
        }
        let mut offset = 0;
        loop {
            const NAME: usize = offset_of!(FILE_ID_BOTH_DIR_INFO, FileName);
            let length = size_of_val(buffer.as_slice()) - offset;
            if length < NAME {
                return Err(invalid(UNAVAILABLE));
            }
            // The API writes an aligned linked record sequence into the buffer.
            let start = unsafe { buffer.as_ptr().cast::<u8>().add(offset) };
            let entry = start.cast::<FILE_ID_BOTH_DIR_INFO>();
            let (bytes, attr, next) = unsafe {
                (
                    (*entry).FileNameLength as usize,
                    (*entry).FileAttributes,
                    (*entry).NextEntryOffset as usize,
                )
            };
            if !bytes.is_multiple_of(2) || NAME + bytes > length {
                return Err(invalid(UNAVAILABLE));
            }
            let units =
                unsafe { std::slice::from_raw_parts(start.add(NAME).cast::<u16>(), bytes / 2) };
            let name = OsString::from_wide(units);
            if name != "." && name != ".." {
                items.push((
                    path.join(&name),
                    attr & FILE_ATTRIBUTE_DIRECTORY != 0,
                    attr & FILE_ATTRIBUTE_REPARSE_POINT != 0 || !valid_name(&name),
                ));
                if items.len() >= limit {
                    return Ok(items);
                }
            }
            if next == 0 {
                break;
            }
            if next < NAME + bytes || next > length || !next.is_multiple_of(8) {
                return Err(invalid(UNAVAILABLE));
            }
            offset += next;
        }
    }
}

fn rename(file: &File, parent: &File, target: &OsStr) -> io::Result<()> {
    if !valid_name(target) {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    let name = target.encode_wide().collect::<Vec<_>>();
    let name_bytes = name.len() * 2;
    let length = size_of::<FILE_RENAME_INFORMATION>() + name_bytes;
    // The trailing UTF-16 name belongs to the native variable-length record.
    // Zero initialization selects ReplaceIfExists = FALSE.
    let mut storage = vec![0usize; length.div_ceil(size_of::<usize>())];
    let mut ios = IO_STATUS_BLOCK::default();
    unsafe {
        let record = storage.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
        (*record).RootDirectory = parent.as_raw_handle();
        (*record).FileNameLength = name_bytes as u32;
        std::ptr::copy_nonoverlapping(
            name.as_ptr(),
            std::ptr::addr_of_mut!((*record).FileName).cast(),
            name.len(),
        );
        status_result(NtSetInformationFile(
            file.as_raw_handle(),
            &mut ios,
            record.cast(),
            length as u32,
            FileRenameInformation,
        ))
    }
}

fn discard(file: &File) {
    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
    unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle(),
            FileDispositionInfo,
            (&raw const disposition).cast(),
            size_of::<FILE_DISPOSITION_INFO>() as u32,
        );
    }
}

pub(super) fn export_destination(parent: &Path, name: &OsStr) -> Result<File> {
    let parent = directory(parent, FILE_ADD_FILE, None)?;
    child(&parent, name, false, FILE_GENERIC_WRITE, FILE_CREATE).map_err(export_error)
}

fn export_error(error: io::Error) -> crate::Error {
    invalid(if error.kind() == io::ErrorKind::AlreadyExists {
        "PROJECT_RESULT_EXPORT_EXISTS"
    } else {
        "PROJECT_RESULT_EXPORT_FAILED"
    })
}

pub(crate) struct ExportDestination {
    file: File,
    directory: File,
    target: OsString,
    committed: bool,
}

// A substituted drive can start below the private directory without naming
// that directory during traversal. Compare the actual local volume paths
// returned for both open handles as well as the file IDs checked on the walk.
fn volume_path(file: &File) -> Result<PathBuf> {
    let mut path = vec![0u16; 256];
    loop {
        let len = unsafe {
            GetFinalPathNameByHandleW(
                file.as_raw_handle(),
                path.as_mut_ptr(),
                path.len() as u32,
                VOLUME_NAME_GUID | FILE_NAME_NORMALIZED,
            )
        } as usize;
        if len == 0 || len > 32768 {
            return Err(invalid("PROJECT_RESULT_EXPORT_PATH_INVALID"));
        }
        if len < path.len() {
            return Ok(PathBuf::from(OsString::from_wide(&path[..len])));
        }
        path.resize(len + 1, 0);
    }
}

pub(super) fn stage_for_owner(
    parent: &Path,
    target: &OsStr,
    private: &Path,
) -> Result<ExportDestination> {
    if !valid_name(target) {
        return Err(invalid("PROJECT_RESULT_EXPORT_PATH_INVALID"));
    }
    let private = directory(&std::fs::canonicalize(private)?, DIRECTORY_ACCESS, None)?;
    let directory = directory(parent, FILE_ADD_FILE, Some(identity(&private)?))?;
    if volume_path(&directory)?.starts_with(volume_path(&private)?) {
        return Err(invalid("PROJECT_RESULT_EXPORT_PATH_INVALID"));
    }
    let temporary = format!(".moxcrew-export-{}.tmp", uuid::Uuid::new_v4());
    let file = child(
        &directory,
        OsStr::new(&temporary),
        false,
        FILE_GENERIC_WRITE | DELETE,
        FILE_CREATE,
    )
    .map_err(export_error)?;
    Ok(ExportDestination {
        file,
        directory,
        target: target.to_owned(),
        committed: false,
    })
}

impl Write for ExportDestination {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.file.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
impl ExportDestination {
    pub(crate) fn commit(mut self) -> Result<()> {
        self.file.flush()?;
        self.file.sync_all()?;
        rename(&self.file, &self.directory, &self.target).map_err(export_error)?;
        self.committed = true;
        Ok(())
    }
}
impl Drop for ExportDestination {
    fn drop(&mut self) {
        if !self.committed {
            discard(&self.file);
        }
    }
}

pub(crate) fn publish_outputs(
    root: &Path,
    folder: &str,
    files: &[(String, Vec<u8>)],
) -> Result<()> {
    if !folder.starts_with("tool-")
        || !folder
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || files
            .iter()
            .any(|(name, _)| name.starts_with('.') || !valid_name(OsStr::new(name)))
    {
        return Err(invalid("PRODUCTION_OUTPUT_INVALID"));
    }
    let parent = directory(&root.join("outputs"), FILE_ADD_SUBDIRECTORY, None)?;
    let temporary = format!(".moxcrew-tool-{}", uuid::Uuid::new_v4());
    let output = child(
        &parent,
        OsStr::new(&temporary),
        true,
        DIRECTORY_ACCESS | FILE_ADD_FILE | DELETE,
        FILE_CREATE,
    )?;
    let mut written = Vec::new();
    let mut created = Vec::new();
    let result = (|| {
        for (name, bytes) in files {
            let file = child(
                &output,
                OsStr::new(name),
                false,
                FILE_GENERIC_WRITE | DELETE,
                FILE_CREATE,
            )?;
            written.push(file);
            let file = written.last_mut().unwrap();
            created.push((name, identity(file)?));
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        // Windows refuses a directory rename while its children deny delete sharing.
        written.clear();
        rename(&output, &parent, OsStr::new(folder)).map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                invalid("PRODUCTION_OUTPUT_CONFLICT")
            } else {
                invalid("PRODUCTION_OUTPUT_UNSAFE")
            }
        })?;
        Ok(())
    })();
    if result.is_err() {
        for file in &written {
            discard(file);
        }
        drop(written);
        // After a rename collision, remove only file identities that we created.
        for (name, original) in created {
            if let Ok(file) = child(&output, OsStr::new(name), false, DELETE, FILE_OPEN) {
                if identity(&file).ok() == Some(original) {
                    discard(&file);
                }
            }
        }
        discard(&output);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn producer_publishes_complete_folder_and_cleans_up_a_collision() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join("outputs")).unwrap();
        let inputs = vec![
            ("Özet.txt".to_owned(), b"first result".to_vec()),
            ("plot.bin".to_owned(), vec![0, 255]),
        ];
        publish_outputs(root, "tool-proof", &inputs).unwrap();
        let outputs = root.join("outputs");
        assert_eq!(
            fs::read(outputs.join("tool-proof/Özet.txt")).unwrap(),
            inputs[0].1
        );
        let different = vec![("Özet.txt".to_owned(), b"second result".to_vec())];
        assert!(publish_outputs(root, "tool-proof", &different).is_err());
        let duplicates = vec![("same.txt".into(), vec![1]), ("SAME.TXT".into(), vec![2])];
        assert!(publish_outputs(root, "tool-duplicate", &duplicates).is_err());
        assert_eq!(
            fs::read(outputs.join("tool-proof/Özet.txt")).unwrap(),
            inputs[0].1
        );
        assert_eq!(fs::read_dir(outputs).unwrap().count(), 1);
    }

    #[test]
    fn windows_names_links_and_internal_export_destinations_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let safe = root.join("private");
        fs::create_dir(&safe).unwrap();
        fs::create_dir(safe.join("sub")).unwrap();
        fs::write(safe.join("proof.txt"), b"one").unwrap();
        for name in [
            "proof.txt:stream",
            "AUX.txt",
            "CONOUT$",
            "LPT¹.txt",
            "trailing.",
            "trailing ",
            "child/name",
        ] {
            assert!(!valid_name(OsStr::new(name)), "{name}");
            assert!(
                stage_for_owner(root, OsStr::new(name), &safe).is_err(),
                "{name}"
            );
        }
        fs::hard_link(safe.join("proof.txt"), root.join("shared.txt")).unwrap();
        assert!(open_result(root, Path::new("shared.txt")).is_err());
        assert!(stage_for_owner(&root.join("PrIvAtE/sub"), OsStr::new("blocked"), &safe).is_err());
        // Junctions require no developer-mode permission on Windows. The fixed
        // command and names operate only within this disposable fixture.
        let junction = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J", "alias", "private"])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(junction.status.success(), "{:?}", junction.status);
        assert!(open_result(root, Path::new("alias/proof.txt")).is_err());
        assert!(entries(&root.join("alias"), 20).is_err());
        assert!(stage_for_owner(&root.join("alias/sub"), OsStr::new("escape"), &safe).is_err());
        fs::remove_dir(root.join("alias")).unwrap();
    }

    #[test]
    fn owner_target_stays_invisible_until_commit_and_a_competing_file_survives() {
        let temp = tempfile::tempdir().unwrap();
        let private = temp.path().join("private");
        let parent = temp.path().join("owner");
        fs::create_dir(&private).unwrap();
        fs::create_dir(&parent).unwrap();
        let target = parent.join("final.md");
        let mut draft = stage_for_owner(&parent, OsStr::new("final.md"), &private).unwrap();
        draft.write_all(b"draft").unwrap();
        assert!(!target.exists());
        drop(draft);
        assert_eq!(fs::read_dir(&parent).unwrap().count(), 0);
        let mut draft = stage_for_owner(&parent, OsStr::new("final.md"), &private).unwrap();
        draft.write_all(b"complete").unwrap();
        fs::write(&target, b"other writer").unwrap();
        assert_eq!(
            draft.commit().unwrap_err().0,
            "PROJECT_RESULT_EXPORT_EXISTS"
        );
        assert_eq!(fs::read(target).unwrap(), b"other writer");
        assert_eq!(fs::read_dir(parent).unwrap().count(), 1);
    }
}
