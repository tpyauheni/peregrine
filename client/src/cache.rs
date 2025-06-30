use std::{
    path::PathBuf,
    sync::LazyLock,
};

use dioxus::signals::{Signal, Writable};
use platform_dirs::AppDirs;
use server::{AccountCredentials, MultiUserGroup, UserAccount};

use crate::{future_retry_loop, packet_sender::{PacketSender, PacketState}};
use shared::{storage::{GeneralStorage, RawStorage}, types::UserIcon};

pub static FALLBACK_CACHE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut path = PathBuf::new();
    path.push("peregrine");
    path.push("cache");
    path
});

pub struct CacheStorage {
    base_path: PathBuf,
}

impl Default for CacheStorage {
    fn default() -> Self {
        let cache_dir = AppDirs::new(Some("peregrine"), false)
            .map_or(FALLBACK_CACHE_PATH.to_path_buf(), |dirs| dirs.cache_dir);
        Self {
            base_path: cache_dir,
        }
    }
}

impl RawStorage for CacheStorage {
    fn get_base_path(&self) -> &PathBuf {
        &self.base_path
    }
}

impl GeneralStorage for CacheStorage {}

impl CacheStorage {
    pub fn store_user_data(&self, user_id: u64, data: &UserAccount) {
        self.store(&format!("user{user_id}.bin"), data);
    }

    pub fn load_user_data(&self, user_id: u64) -> Option<UserAccount> {
        self.load(&format!("user{user_id}.bin"))
    }

    pub fn store_group_data(&self, group_id: u64, data: &MultiUserGroup) {
        self.store(&format!("group{group_id}.bin"), data);
    }

    pub fn load_group_data(&self, group_id: u64) -> Option<MultiUserGroup> {
        self.load(&format!("group{group_id}.bin"))
    }

    pub async fn user_data(&self, user_id: u64, credentials: AccountCredentials, signal: &mut Signal<PacketState<Option<UserAccount>>>) {
        if let Some(data) = self.load_user_data(user_id) {
            signal.set(PacketState::Response(Some(data)));
            return;
        }

        PacketSender::default()
            .retry_loop(|| server::get_user_data(user_id, credentials), signal)
            .await;

        if let PacketState::Response(Some(ref data)) = signal() {
            self.store_user_data(user_id, data);
        }
    }

    pub async fn group_data(&self, group_id: u64, credentials: AccountCredentials, signal: &mut Signal<PacketState<Option<MultiUserGroup>>>) {
        if let Some(data) = self.load_group_data(group_id) {
            signal.set(PacketState::Response(Some(data)));
            return;
        }

        PacketSender::default()
            .retry_loop(|| server::get_group_data(group_id, credentials), signal)
            .await;

        if let PacketState::Response(Some(ref data)) = signal() {
            self.store_group_data(group_id, data);
        }
    }
}

pub static CACHE: LazyLock<CacheStorage> = LazyLock::new(Default::default);
