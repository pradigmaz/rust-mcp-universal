use std::fs;
use std::io::Write;

use super::RebuildLockGuard;
use super::acquire::OperationLockGuard;
use super::metadata::lock_belongs_to_token;

impl Drop for RebuildLockGuard {
    fn drop(&mut self) {
        let _ = self.lock_file.flush();
        if let Ok(_op_guard) = OperationLockGuard::acquire(&self.lock_path) {
            if lock_belongs_to_token(&self.lock_path, &self.lock_token) {
                let _ = fs::remove_file(&self.lock_path);
            }
        }
    }
}
