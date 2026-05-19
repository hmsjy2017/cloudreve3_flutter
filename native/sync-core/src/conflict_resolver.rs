use crate::models::*;

#[derive(Clone)]
pub struct ConflictResolver {
    pub default_strategy: ConflictStrategy,
}

impl ConflictResolver {
    pub fn new(strategy: ConflictStrategy) -> Self {
        Self {
            default_strategy: strategy,
        }
    }

    pub fn resolve(
        &self,
        conflict_type: ConflictType,
        local_mtime: i64,
        remote_mtime: i64,
        local_size: u64,
        remote_size: u64,
        local_name: &str,
    ) -> ConflictResolution {
        self.resolve_with_strategy(
            &self.default_strategy,
            conflict_type,
            local_mtime,
            remote_mtime,
            local_size,
            remote_size,
            local_name,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resolve_with_strategy(
        &self,
        strategy: &ConflictStrategy,
        _conflict_type: ConflictType,
        local_mtime: i64,
        remote_mtime: i64,
        local_size: u64,
        remote_size: u64,
        local_name: &str,
    ) -> ConflictResolution {
        match strategy {
            ConflictStrategy::KeepLocal => ConflictResolution::UploadLocal,
            ConflictStrategy::KeepRemote => ConflictResolution::DownloadRemote,
            ConflictStrategy::KeepBoth => {
                let new_name = crate::utils::generate_conflict_name(local_name);
                ConflictResolution::RenameLocal { new_name }
            }
            ConflictStrategy::NewestWins => {
                if local_mtime > remote_mtime {
                    ConflictResolution::UploadLocal
                } else {
                    ConflictResolution::DownloadRemote
                }
            }
            ConflictStrategy::LargestWins => {
                if local_size > remote_size {
                    ConflictResolution::UploadLocal
                } else {
                    ConflictResolution::DownloadRemote
                }
            }
            ConflictStrategy::Manual => ConflictResolution::MarkManual,
        }
    }
}
