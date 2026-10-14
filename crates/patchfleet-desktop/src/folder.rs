use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Registration,
    Task,
}

#[derive(Clone, Debug)]
pub struct Selection {
    pub path: String,
    pub problem: Option<String>,
}

pub async fn check(path: PathBuf) -> Selection {
    let Some(path) = path.to_str() else {
        return Selection {
            path: String::new(),
            problem: Some("WORKSPACE_PATH_INVALID".into()),
        };
    };
    match patchfleet_core::providers::validate_workspace_for_registration(path).await {
        Ok(canonical) => Selection {
            path: canonical,
            problem: None,
        },
        Err(code) => Selection {
            path: path.to_owned(),
            problem: Some(crate::command::safe_error(code)),
        },
    }
}

pub fn is_problem(code: &str) -> bool {
    matches!(
        code,
        "WORKSPACE_PATH_INVALID"
            | "WORKSPACE_FOLDER_NOT_FOUND"
            | "WORKSPACE_NOT_DIRECTORY"
            | "WORKSPACE_UNSAFE_ROOT"
            | "WORKSPACE_NOT_GIT_ROOT"
            | "WORKSPACE_ACCESS_DENIED"
            | "WORKSPACE_READ_ONLY"
            | "WORKSPACE_UNAVAILABLE"
            | "WORKSPACE_NOT_ALLOWED"
    )
}

pub fn permission_guidance(code: &str) -> &'static str {
    if cfg!(target_os = "macos") && code == "WORKSPACE_READ_ONLY" {
        "In Finder, open the folder's Get Info > Sharing & Permissions and allow your account to Read & Write. If the drive is read-only, choose a folder on a writable drive."
    } else if cfg!(target_os = "macos") {
        "Select the folder again and allow access if macOS asks. If access was denied, enable Patchfleet in System Settings > Privacy & Security > Files and Folders."
    } else if cfg!(target_os = "windows") {
        "Select the folder again. In the folder's Properties > Security, allow your account to read and modify it. If Windows blocked an agent, review that agent's access separately."
    } else {
        "Select the folder again. Check your account's read, write and traversal permissions in the file manager. A read-only drive must be mounted writable."
    }
}
