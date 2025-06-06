use crate::{Account, DmMessage};

use std::sync::{LazyLock, Arc, Mutex};

use mysql::{params, Pool, PooledConn};
use mysql::prelude::*;
use rand::{rngs::StdRng, SeedableRng};

#[derive(Debug, Clone)]
pub struct Database {
    pool: Pool,
}

type DbResult<T> = Result<T, Box<dyn std::error::Error>>;

impl Database {
    pub fn try_new(url: &str) -> DbResult<Self> {
        Ok(Self {
            pool: Pool::new(url)?,
        })
    }

    pub fn new(url: &str) -> Self {
        Self::try_new(url).unwrap()
    }

    pub fn get_context(&self) -> DbResult<PooledConn> {
        Ok(self.pool.get_conn()?)
    }

    pub fn init(&self) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        conn.query_drop(r"
            CREATE TABLE IF NOT EXISTS `accounts` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `public_key` BLOB NOT NULL,
                `encrypted_private_info` BLOB NOT NULL,
                `email` VARCHAR(255),
                `username` VARCHAR(255)
            );
        ")?;
        conn.query_drop(r"
            CREATE TABLE IF NOT EXISTS `sessions` (
                `account_id` BIGINT NOT NULL,
                `session_token` BLOB NOT NULL,
                `begin_time` DATETIME NOT NULL,
                `end_time` DATETIME NOT NULL
            );
        ")?;
        conn.query_drop(r"
            CREATE TABLE IF NOT EXISTS `groups` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `name` VARCHAR(255),
                `icon` BLOB NOT NULL,
                `encrypted` BIT NOT NULL,
                `public` BIT NOT NULL,
                `channel` BIT NOT NULL
            );
        ")?;
        conn.query_drop(r"
            CREATE TABLE IF NOT EXISTS `dm_groups` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `encrypted` BIT NOT NULL,
                `initiator_id` BIGINT NOT NULL,
                `other_id` BIGINT NOT NULL
            );
        ")?;
        // Table `group_members` is not intended for channel members (which are not stored on the
        // server) and it's not intended for DM groups.
        conn.query_drop(r"
            CREATE TABLE IF NOT EXISTS `group_members` (
                `group_id` BIGINT NOT NULL,
                `user_id` BIGINT NOT NULL,
                `permissions` VARCHAR(255) NOT NULL
            );
        ")?;
        conn.query_drop(r"
            CREATE TABLE IF NOT EXISTS `messages` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `sender_id` BIGINT NOT NULL,
                `group_id` BIGINT NOT NULL,
                `encryption_method` VARCHAR(16) NOT NULL,
                `reply_message_id` BIGINT,
                `edited_message_id` BIGINT,
                `content` BLOB NOT NULL,
                `send_time` DATETIME NOT NULL,
                `is_dm` BIT NOT NULL
            );
        ")?;
        conn.query_drop(r"
            CREATE TABLE IF NOT EXISTS `read_messages` (
                `message_id` BIGINT NOT NULL,
                `user_id` BIGINT NOT NULL,
                `timestamp` DATETIME DEFAULT CURRENT_TIMESTAMP
            );
        ")?;
        Ok(())
    }

    pub fn create_account(
        &self,
        public_key: &[u8],
        encrypted_private_info: &[u8],
        email: Option<&str>,
        username: Option<&str>,
    ) -> DbResult<u64> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"INSERT INTO `accounts` (
                `public_key`,
                `encrypted_private_info`,
                `email`,
                `username`
            ) VALUES (?, ?, ?, ?);",
            (
                public_key,
                encrypted_private_info,
                email,
                username,
            ),
        )?;
        // `LAST_INSERT_ID()` returns the last id only for the current Pool connection. 
        Ok(conn.query_first("SELECT LAST_INSERT_ID();")?.unwrap())
    }

    pub fn create_session(
        &self,
        account_id: u64,
        begin_time: Option<chrono::NaiveDateTime>,
        end_time: Option<chrono::NaiveDateTime>,
    ) -> DbResult<[u8; 32]> {
        let mut session_token = [0u8; 32];
        rng::fill_bytes(&mut session_token);
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"INSERT INTO `sessions` (
                `account_id`,
                `session_token`,
                `begin_time`,
                `end_time`
            ) VALUES (?, ?, IFNULL(?, CURRENT_TIMESTAMP()), IFNULL(?, DATE_ADD(NOW(), INTERVAL 7 DAY)));",
            (
                account_id,
                session_token,
                begin_time,
                end_time,
            ),
        )?;
        Ok(session_token)
    }

    pub fn find_user(&self, query: &str) -> DbResult<Vec<Account>> {
        let mut conn = self.pool.get_conn()?;
        let query = format!("%{query}%");
        let accounts = conn.exec_map(
            r"SELECT * FROM `accounts`
                WHERE `username` LIKE :query
                    OR `email` LIKE :query
                LIMIT 10;",
            params! {
                query,
            },
            |(id, public_key, encrypted_private_info, email, username)| {
                Account { id, public_key, encrypted_private_info, email, username }
            }
        )?;
        Ok(accounts)
    }

    pub fn is_session_valid(
        &self,
        account_id: u64,
        session_token: [u8; 32],
    ) -> DbResult<bool> {
        let mut conn = self.pool.get_conn()?;
        let value: Option<u64> = conn.exec_first(
            r"SELECT `account_id` FROM `sessions`
                WHERE `account_id` = ?
                AND `session_token` = ?
                AND `begin_time` <= NOW()
                AND `end_time` > NOW()
                LIMIT 1;",
            (account_id, session_token),
        )?;
        Ok(value.is_some())
    }

    pub fn create_dm_group(
        &self,
        initiator_id: u64,
        other_id: u64,
        encrypted: bool,
    ) -> DbResult<u64> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(r"INSERT INTO `dm_groups` (`initiator_id`, `other_id`, `encrypted`)
                VALUES (?, ?, ?);",
            (
                initiator_id,
                other_id,
                encrypted,
            ),
        )?;
        // `LAST_INSERT_ID()` returns the last id only for the current Pool connection. 
        let group_id: u64 = conn.query_first("SELECT LAST_INSERT_ID();")?.unwrap();
        Ok(group_id)
    }

    pub fn send_dm_message(
        &self,
        sender_id: u64,
        group_id: u64,
        encryption_method: &str,
        content: &[u8],
        send_time: Option<chrono::NaiveDateTime>,
    ) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        // `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
        // `group_id` BIGINT NOT NULL,
        // `encryption_method` VARCHAR(16) NOT NULL,
        // `reply_message_id` BIGINT,
        // `edited_message_id` BIGINT,
        // `content` BLOB NOT NULL,
        // `send_time` DATETIME NOT NULL,
        // `is_dm` BIT NOT NULL
        conn.exec_drop(r"INSERT INTO `messages` (
                `group_id`,
                `sender_id`,
                `encryption_method`,
                `reply_message_id`,
                `edited_message_id`,
                `content`,
                `send_time`,
                `is_dm`
            ) VALUES (?, ?, ?, NULL, NULL, ?, IFNULL(?, CURRENT_TIMESTAMP()), 1)",
            (
                group_id,
                sender_id,
                encryption_method,
                content,
                send_time,
            ),
        )?;
        Ok(())
    }

    pub fn get_dm_messages(
        &self,
        group_id: u64,
        account_id: u64,
    ) -> DbResult<Vec<DmMessage>> {
        let mut conn = self.pool.get_conn()?;
        let value = conn.exec_map(
            r"SELECT
                `id`,
                `sender_id`,
                `encryption_method`,
                `reply_message_id`,
                `edited_message_id`,
                `content`,
                `send_time`
                FROM `messages`
                WHERE `group_id` = ?
                AND `is_dm` = 1
                ORDER BY `send_time` DESC
                LIMIT 30;",
            (group_id,),
            |(
                id,
                sender_id,
                encryption_method,
                reply_message_id,
                edited_message_id,
                content,
                send_time,
            )| {
                let _: u64 = sender_id;
                DmMessage {
                    id,
                    sent_by_me: sender_id == account_id,
                    encryption_method,
                    content,
                    reply_to: reply_message_id,
                    edit_for: edited_message_id,
                    sent_time: send_time,
                }
            },
        )?;
        Ok(value)
    }

    /// # Safety:
    /// Always safe. Marked as unsafe only to prevent complete data resets (e.g. when the wrong
    /// option is selected in a code autocomplion menu).
    pub unsafe fn reset(&self) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        conn.query_drop("DROP TABLE IF EXISTS `accounts`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `sessions`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `groups`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `dm_groups`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `group_members`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `messages`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `read_messages`;")?;
        self.init()?;
        Ok(())
    }
}

static RNG: LazyLock<Arc<Mutex<StdRng>>> = LazyLock::new(||
    Arc::new(Mutex::new(StdRng::from_os_rng()))
);
pub static DB: LazyLock<Database> = LazyLock::new(||
    Database::new(&std::env::var("DB_URL").unwrap())
);

// TODO: Move into another module
pub mod rng {
    use std::sync::{Arc, Mutex};
    use rand::{rngs::StdRng, RngCore};
    use super::RNG;

    pub fn get_rng() -> Arc<Mutex<StdRng>> {
        RNG.clone()
    }

    pub fn fill_bytes(destination: &mut [u8]) {
        get_rng().lock().unwrap().fill_bytes(destination);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{LazyLock, Once};

    use crate::secret::db::Account;

    use super::Database;

    static DB: LazyLock<Database> = LazyLock::new(||
        Database::new(&std::env::var("TEST_DB_URL").unwrap())
    );
    static INIT: Once = Once::new();

    fn init() {
        INIT.call_once(|| {
            unsafe { DB.reset() }.unwrap();
        });
    }

    #[test]
    fn db_test_users() {
        init();

        DB.create_account(
            &[1],
            &[],
            Some("some_email@example.com"),
            Some("The first User"),
        ).unwrap();
        DB.create_account(
            &[2],
            &[],
            None,
            Some("The second user"),
        ).unwrap();
        DB.create_account(
            &[3],
            &[],
            Some("third_user@example.com"),
            None,
        ).unwrap();
        DB.create_account(
            &[4],
            &[],
            None,
            None,
        ).unwrap();
        DB.create_account(
            &[5],
            &[],
            Some("different_account@example.com"),
            Some("Account 5"),
        ).unwrap();
        assert_eq!(
            DB.find_user("user").unwrap(),
            vec![
                Account {
                    id: 1,
                    public_key: Box::new([1]),
                    encrypted_private_info: Box::new([]),
                    email: Some("some_email@example.com".to_owned()),
                    username: Some("The first User".to_owned()),
                },
                Account {
                    id: 2,
                    public_key: Box::new([2]),
                    encrypted_private_info: Box::new([]),
                    email: None,
                    username: Some("The second user".to_owned()),
                },
                Account {
                    id: 3,
                    public_key: Box::new([3]),
                    encrypted_private_info: Box::new([]),
                    email: Some("third_user@example.com".to_owned()),
                    username: None,
                },
            ],
        );
        let token = DB.create_session(1, None, None).unwrap();
        assert!(DB.is_session_valid(1, token).unwrap());
        assert!(!DB.is_session_valid(2, token).unwrap());
        assert!(!DB.is_session_valid(3, token).unwrap());
        let token2 = DB.create_session(2, None, None).unwrap();
        assert!(!DB.is_session_valid(1, token2).unwrap());
        assert!(DB.is_session_valid(2, token2).unwrap());
        assert!(!DB.is_session_valid(3, token2).unwrap());
        let dm_group1 = DB.create_dm_group(1, 2, true).unwrap();
        assert_eq!(dm_group1, 1);
        DB.send_dm_message(1, dm_group1, "!plaintext", "Hello, World!".as_bytes(), None).unwrap();
        DB.send_dm_message(2, dm_group1, "privatecipher123", &[0x69, 0x68], None).unwrap();
        let dm_messages1 = DB.get_dm_messages(dm_group1, 1).unwrap();
        assert_eq!(dm_messages1[0].id, 1);
        assert_eq!(dm_messages1[0].encryption_method, "!plaintext");
        assert_eq!(dm_messages1[0].content, "Hello, World!".as_bytes().into());
        assert_eq!(dm_messages1[0].reply_to, None);
        assert_eq!(dm_messages1[0].edit_for, None);
        assert!(dm_messages1[0].sent_by_me);
        assert_eq!(dm_messages1[1].id, 2);
        assert_eq!(dm_messages1[1].encryption_method, "privatecipher123");
        assert_eq!(dm_messages1[1].content, [0x69, 0x68].into());
        assert_eq!(dm_messages1[1].reply_to, None);
        assert_eq!(dm_messages1[1].edit_for, None);
        assert!(!dm_messages1[1].sent_by_me);
        let mut dm_messages2 = DB.get_dm_messages(dm_group1, 2).unwrap();
        dm_messages2[0].sent_by_me = !dm_messages2[0].sent_by_me;
        dm_messages2[1].sent_by_me = !dm_messages2[1].sent_by_me;
        assert_eq!(dm_messages1, dm_messages2);
    }
}
