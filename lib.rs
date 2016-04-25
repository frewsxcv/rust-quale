use std::{env, ffi, fs, path};

#[cfg(all(unix, not(target_os = "linux")))]
use std::os::unix::fs::PermissionsExt;
#[cfg(target_os = "linux")]
use std::os::linux::fs::PermissionsExt;

extern crate libc;

pub fn which<S: AsRef<ffi::OsStr>>(name: S) -> Option<path::PathBuf> {
    let name: &ffi::OsStr = name.as_ref();

    // FIXME: This should use `env::var_os`, but `OsStr` doesn't implement
    //        any sort of 'splitting' method.
    //        https://github.com/rust-lang/rfcs/pull/1309
    let var = match env::var("PATH") {
        Ok(var) => var,
        Err(..) => return None,
    };

    // Separate PATH value into paths
    let paths_iter = var.split(":");

    // Attempt to read each path as a directory
    let dirs_iter = paths_iter.filter_map(|path| fs::read_dir(path).ok());

    for dir in dirs_iter {
        //
        let mut matches_iter = dir.filter_map(|file| file.ok())
                                  .filter(|file| file.file_name() == name)
                                  .filter(is_executable);
        if let Some(file) = matches_iter.next() {
            return Some(file.path());
        }
    }

    None
}

fn is_executable(file: &fs::DirEntry) -> bool {
    let file_metadata = match file.metadata() {
        Ok(metadata) => metadata,
        Err(..) => return false,
    };
    let file_type = match file.file_type() {
        Ok(type_) => type_,
        Err(..) => return false,
    };
    let file_path = match file.path()
                              .to_str()
                              .and_then(|p| ffi::CString::new(p).ok()) {
        Some(path) => path,
        None => return false,
    };
    let is_executable_by_user = unsafe {
        libc::access(file_path.into_raw(), libc::X_OK) == libc::EXIT_SUCCESS
    };
    static EXECUTABLE_FLAGS: u32 =
        (libc::S_IEXEC | libc::S_IXGRP | libc::S_IXOTH) as u32;
    let has_executable_flag =
        file_metadata.permissions().mode() & EXECUTABLE_FLAGS != 0;
    is_executable_by_user && has_executable_flag && file_type.is_file()
}

#[cfg(test)]
mod tests {
    use std::path;
    use super::which;

    /// FIXME: this is not a good test since it relies on PATH and the
    ///        filesystem being in a certain state.
    #[test]
    fn test_sh() {
        let expected = path::PathBuf::from("/bin/sh");
        let actual = which("sh");
        assert_eq!(Some(expected), actual);
    }

    #[test]
    fn test_none() {
        let actual = which("foofoofoobar");
        assert_eq!(None, actual);
    }
}
