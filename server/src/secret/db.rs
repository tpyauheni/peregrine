use crate::{Account, DmInvite, DmMessage};

use std::sync::{Arc, LazyLock, Mutex};

use mysql::prelude::*;
use mysql::{Pool, Row, params};
use rand::{SeedableRng, rngs::StdRng};
use shared::limits::LIMITS;

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

    pub fn init(&self) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        conn.query_drop(
            r"
            CREATE TABLE IF NOT EXISTS `accounts` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `public_key` BLOB NOT NULL,
                `encrypted_private_info` BLOB NOT NULL,
                `email` VARCHAR(255),
                `username` VARCHAR(255)
            );
        ",
        )?;
        conn.query_drop(
            r"
            CREATE TABLE IF NOT EXISTS `sessions` (
                `account_id` BIGINT NOT NULL,
                `session_token` BLOB NOT NULL,
                `begin_time` DATETIME NOT NULL,
                `end_time` DATETIME NOT NULL
            );
        ",
        )?;
        conn.query_drop(
            r"
            CREATE TABLE IF NOT EXISTS `groups` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `name` VARCHAR(255),
                `icon` BLOB NOT NULL,
                `encrypted` BIT NOT NULL,
                `public` BIT NOT NULL,
                `channel` BIT NOT NULL
            );
        ",
        )?;
        conn.query_drop(
            r"
            CREATE TABLE IF NOT EXISTS `dm_groups` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `encrypted` BIT NOT NULL,
                `initiator_id` BIGINT NOT NULL,
                `other_id` BIGINT NOT NULL
            );
        ",
        )?;
        // Table `group_members` is not intended for channel members (which are not stored on the
        // server) and it's not intended for DM groups.
        conn.query_drop(
            r"
            CREATE TABLE IF NOT EXISTS `group_members` (
                `group_id` BIGINT NOT NULL,
                `user_id` BIGINT NOT NULL,
                `permissions` VARCHAR(255) NOT NULL
            );
        ",
        )?;
        conn.query_drop(format!(
            r"
            CREATE TABLE IF NOT EXISTS `messages` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `sender_id` BIGINT NOT NULL,
                `group_id` BIGINT NOT NULL,
                `encryption_method` VARCHAR({}) NOT NULL,
                `reply_message_id` BIGINT,
                `edited_message_id` BIGINT,
                `content` BLOB NOT NULL,
                `send_time` DATETIME NOT NULL,
                `is_dm` BIT NOT NULL
            );
        ",
            LIMITS.max_encryption_method_length
        ))?;
        conn.query_drop(
            r"
            CREATE TABLE IF NOT EXISTS `read_messages` (
                `message_id` BIGINT NOT NULL,
                `user_id` BIGINT NOT NULL,
                `timestamp` DATETIME DEFAULT CURRENT_TIMESTAMP
            );
        ",
        )?;
        conn.query_drop(
            r"
            CREATE TABLE IF NOT EXISTS `dm_invites` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `initiator_id` BIGINT NOT NULL,
                `other_id` BIGINT NOT NULL,
                `encrypted` BIT NOT NULL
            );
        ",
        )?;
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
            (public_key, encrypted_private_info, email, username),
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
            |(id, public_key, encrypted_private_info, email, username)| Account {
                id,
                public_key,
                encrypted_private_info,
                email,
                username,
            },
        )?;
        Ok(accounts)
    }

    pub fn is_session_valid(&self, account_id: u64, session_token: [u8; 32]) -> DbResult<bool> {
        let mut conn = self.pool.get_conn()?;
        let value: Option<u8> = conn.exec_first(
            r"SELECT 1 FROM `sessions`
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
        conn.exec_drop(
            r"INSERT INTO `dm_groups` (`initiator_id`, `other_id`, `encrypted`)
                VALUES (?, ?, ?);",
            (initiator_id, other_id, encrypted),
        )?;
        // `LAST_INSERT_ID()` returns the last id only for the current Pool connection.
        let group_id: u64 = conn.query_first("SELECT LAST_INSERT_ID();")?.unwrap();
        Ok(group_id)
    }

    pub fn is_in_dm_group(&self, sender_id: u64, group_id: u64) -> DbResult<bool> {
        let mut conn = self.pool.get_conn()?;
        let value: Option<u8> = conn.exec_first(
            r"SELECT 1 FROM `dm_groups`
                WHERE (`initiator_id` = :sender_id
                    OR `other_id` = :sender_id)
                    AND `id` = :group_id;",
            params! {
                group_id,
                sender_id,
            },
        )?;
        Ok(value.is_some())
    }

    pub fn send_dm_message(
        &self,
        sender_id: u64,
        group_id: u64,
        encryption_method: &str,
        content: &[u8],
        send_time: Option<chrono::NaiveDateTime>,
    ) -> DbResult<u64> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"INSERT INTO `messages` (
                `group_id`,
                `sender_id`,
                `encryption_method`,
                `reply_message_id`,
                `edited_message_id`,
                `content`,
                `send_time`,
                `is_dm`
            ) VALUES (?, ?, ?, NULL, NULL, ?, IFNULL(?, CURRENT_TIMESTAMP()), 1)",
            (group_id, sender_id, encryption_method, content, send_time),
        )?;
        Ok(conn.query_first("SELECT LAST_INSERT_ID();")?.unwrap())
    }

    pub fn get_dm_messages(
        &self,
        first_message_id: u64,
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
                WHERE `id` > ?
                    AND `group_id` = ?
                    AND `is_dm` = 1
                ORDER BY `send_time` DESC
                LIMIT 30;",
            (first_message_id, group_id),
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

    pub fn add_dm_invite(
        &self,
        initiator_id: u64,
        other_id: u64,
        encrypted: bool,
    ) -> DbResult<u64> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"INSERT INTO `dm_invites` (
            `initiator_id`,
            `other_id`,
            `encrypted`
        ) VALUES (?, ?, ?);",
            (initiator_id, other_id, encrypted),
        )?;
        Ok(conn.query_first("SELECT LAST_INSERT_ID();")?.unwrap())
    }

    pub fn get_dm_invite(&self, id: u64) -> DbResult<DmInvite> {
        let mut conn = self.pool.get_conn()?;
        let mut invite: Row = conn
            .exec_first(
                r"SELECT * FROM `invites`
            WHERE `id` = ?;",
                (id,),
            )?
            .unwrap();
        Ok(DmInvite {
            id: invite.take_opt(0).unwrap()?,
            initiator_id: invite.take_opt(1).unwrap()?,
            other_id: invite.take_opt(2).unwrap()?,
            encrypted: invite.take_opt(3).unwrap()?,
        })
    }

    pub fn remove_dm_invite(&self, id: u64) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"DELETE FROM `dm_invites`
            WHERE `id` = ?;",
            (id,),
        )?;
        Ok(())
    }

    pub fn get_sent_dm_invites(&self, id: u64) -> DbResult<Vec<DmInvite>> {
        let mut conn = self.pool.get_conn()?;
        let value = conn.exec_map(
            r"SELECT
                *
                FROM `dm_invites`
                WHERE `initiator_id` = ? 
                ORDER BY `id` DESC
                LIMIT 30;",
            (id,),
            |(id, initiator_id, other_id, encrypted_bytes)| {
                let _: Box<[u8]> = encrypted_bytes;
                DmInvite {
                    id,
                    initiator_id,
                    other_id,
                    encrypted: encrypted_bytes[0] != 0,
                }
            },
        )?;
        Ok(value)
    }

    pub fn get_received_dm_invites(&self, id: u64) -> DbResult<Vec<DmInvite>> {
        let mut conn = self.pool.get_conn()?;
        let value = conn.exec_map(
            r"SELECT
                *
                FROM `dm_invites`
                WHERE `other_id` = ? 
                ORDER BY `id` DESC
                LIMIT 30;",
            (id,),
            |(id, initiator_id, other_id, encrypted_bytes)| {
                let _: Box<[u8]> = encrypted_bytes;
                DmInvite {
                    id,
                    initiator_id,
                    other_id,
                    encrypted: encrypted_bytes[0] != 0,
                }
            },
        )?;
        Ok(value)
    }

    pub fn is_valid_user_id(&self, id: u64) -> DbResult<bool> {
        let mut conn = self.pool.get_conn()?;
        let value: Option<u8> = conn.exec_first(
            r"SELECT 1 FROM `accounts`
            WHERE id = ?;",
            (id,),
        )?;
        Ok(value.is_some())
    }

    pub fn remove_dm_group(&self, group_id: u64) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        Ok(conn.exec_drop(
            r"DELETE FROM `dm_groups`
            WHERE id = ?",
            (group_id,),
        )?)
    }

    #[cfg(test)]
    pub fn reset(&self) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        conn.query_drop("DROP TABLE IF EXISTS `accounts`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `sessions`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `groups`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `dm_groups`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `group_members`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `messages`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `read_messages`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `dm_invites`;")?;
        self.init()?;
        Ok(())
    }
}

static RNG: LazyLock<Arc<Mutex<StdRng>>> =
    LazyLock::new(|| Arc::new(Mutex::new(StdRng::from_os_rng())));
pub static DB: LazyLock<Database> =
    LazyLock::new(|| Database::new(&std::env::var("DB_URL").unwrap()));

// TODO: Move into another module
pub mod rng {
    use super::RNG;
    use rand::{RngCore, rngs::StdRng};
    use std::sync::{Arc, Mutex};

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

    use crate::{DmInvite, secret::db::Account};

    use super::Database;

    static DB: LazyLock<Database> =
        LazyLock::new(|| Database::new(&std::env::var("TEST_DB_URL").unwrap()));
    static INIT: Once = Once::new();

    fn init() {
        INIT.call_once(|| {
            DB.reset().unwrap();
        });
    }

    #[test]
    fn db_test_users() {
        init();

        for id in 0..=6 {
            assert!(!DB.is_valid_user_id(id).unwrap());
        }
        DB.create_account(
            &[1],
            &[],
            Some("some_email@example.com"),
            Some("The first User"),
        )
        .unwrap();
        assert!(!DB.is_valid_user_id(0).unwrap());
        assert!(DB.is_valid_user_id(1).unwrap());
        assert!(!DB.is_valid_user_id(2).unwrap());
        DB.create_account(&[2], &[], None, Some("The second user"))
            .unwrap();
        assert!(!DB.is_valid_user_id(0).unwrap());
        assert!(DB.is_valid_user_id(1).unwrap());
        assert!(DB.is_valid_user_id(2).unwrap());
        assert!(!DB.is_valid_user_id(3).unwrap());
        DB.create_account(&[3], &[], Some("third_user@example.com"), None)
            .unwrap();
        assert!(!DB.is_valid_user_id(0).unwrap());
        assert!(DB.is_valid_user_id(1).unwrap());
        assert!(DB.is_valid_user_id(2).unwrap());
        assert!(DB.is_valid_user_id(3).unwrap());
        assert!(!DB.is_valid_user_id(4).unwrap());
        DB.create_account(&[4], &[], None, None).unwrap();
        assert!(!DB.is_valid_user_id(0).unwrap());
        assert!(DB.is_valid_user_id(1).unwrap());
        assert!(DB.is_valid_user_id(2).unwrap());
        assert!(DB.is_valid_user_id(3).unwrap());
        assert!(DB.is_valid_user_id(4).unwrap());
        assert!(!DB.is_valid_user_id(5).unwrap());
        DB.create_account(
            &[5],
            &[],
            Some("different_account@example.com"),
            Some("Account 5"),
        )
        .unwrap();
        assert!(!DB.is_valid_user_id(0).unwrap());
        assert!(DB.is_valid_user_id(1).unwrap());
        assert!(DB.is_valid_user_id(2).unwrap());
        assert!(DB.is_valid_user_id(3).unwrap());
        assert!(DB.is_valid_user_id(4).unwrap());
        assert!(DB.is_valid_user_id(5).unwrap());
        assert!(!DB.is_valid_user_id(6).unwrap());
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
        let invite1 = DmInvite {
            id: 1,
            initiator_id: 1,
            other_id: 2,
            encrypted: false,
        };
        let invite2 = DmInvite {
            id: 2,
            initiator_id: 3,
            other_id: 2,
            encrypted: false,
        };
        let invite3 = DmInvite {
            id: 3,
            initiator_id: 3,
            other_id: 1,
            encrypted: false,
        };
        DB.add_dm_invite(invite1.initiator_id, invite1.other_id, invite1.encrypted)
            .unwrap();
        DB.add_dm_invite(invite2.initiator_id, invite2.other_id, invite2.encrypted)
            .unwrap();
        DB.add_dm_invite(invite3.initiator_id, invite3.other_id, invite3.encrypted)
            .unwrap();
        assert_eq!(DB.get_sent_dm_invites(1).unwrap(), vec![invite1]);
        assert_eq!(DB.get_received_dm_invites(1).unwrap(), vec![invite3]);
        assert_eq!(DB.get_sent_dm_invites(2).unwrap(), vec![]);
        assert_eq!(
            DB.get_received_dm_invites(2).unwrap(),
            vec![invite2, invite1]
        );
        assert_eq!(DB.get_sent_dm_invites(3).unwrap(), vec![invite3, invite2]);
        assert_eq!(DB.get_received_dm_invites(3).unwrap(), vec![]);
        DB.remove_dm_invite(3).unwrap();
        assert_eq!(DB.get_sent_dm_invites(1).unwrap(), vec![invite1]);
        assert_eq!(DB.get_received_dm_invites(1).unwrap(), vec![]);
        assert_eq!(DB.get_sent_dm_invites(2).unwrap(), vec![]);
        assert_eq!(
            DB.get_received_dm_invites(2).unwrap(),
            vec![invite2, invite1]
        );
        assert_eq!(DB.get_sent_dm_invites(3).unwrap(), vec![invite2]);
        assert_eq!(DB.get_received_dm_invites(3).unwrap(), vec![]);
        assert!(!DB.is_in_dm_group(1, 1).unwrap());
        assert!(!DB.is_in_dm_group(2, 1).unwrap());
        assert!(!DB.is_in_dm_group(3, 1).unwrap());
        assert!(!DB.is_in_dm_group(1, 2).unwrap());
        assert!(!DB.is_in_dm_group(2, 2).unwrap());
        assert!(!DB.is_in_dm_group(3, 2).unwrap());
        let dm_group1 = DB.create_dm_group(1, 2, true).unwrap();
        assert!(DB.is_in_dm_group(1, 1).unwrap());
        assert!(DB.is_in_dm_group(2, 1).unwrap());
        assert!(!DB.is_in_dm_group(3, 1).unwrap());
        assert!(!DB.is_in_dm_group(1, 2).unwrap());
        assert!(!DB.is_in_dm_group(2, 2).unwrap());
        assert!(!DB.is_in_dm_group(3, 2).unwrap());
        assert_eq!(dm_group1, 1);
        DB.send_dm_message(1, dm_group1, "!plaintext", "Hello, World!".as_bytes(), None)
            .unwrap();
        DB.send_dm_message(2, dm_group1, "privatecipher123", &[0x69, 0x68], None)
            .unwrap();
        let dm_messages1 = DB.get_dm_messages(0, dm_group1, 1).unwrap();
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
        assert_eq!(dm_messages1.len(), 2);
        let mut dm_messages2 = DB.get_dm_messages(0, dm_group1, 2).unwrap();
        dm_messages2[0].sent_by_me = !dm_messages2[0].sent_by_me;
        dm_messages2[1].sent_by_me = !dm_messages2[1].sent_by_me;
        assert_eq!(dm_messages1, dm_messages2);
        dm_messages2[0].sent_by_me = !dm_messages2[0].sent_by_me;
        dm_messages2[1].sent_by_me = !dm_messages2[1].sent_by_me;
        let dm_messages3 = DB.get_dm_messages(1, dm_group1, 2).unwrap();
        assert_eq!(dm_messages2[1], dm_messages3[0]);
        assert_eq!(dm_messages3.len(), 1);
        let dm_group2 = DB.create_dm_group(3, 2, true).unwrap();
        assert!(DB.is_in_dm_group(1, 1).unwrap());
        assert!(DB.is_in_dm_group(2, 1).unwrap());
        assert!(!DB.is_in_dm_group(3, 1).unwrap());
        assert!(!DB.is_in_dm_group(1, 2).unwrap());
        assert!(DB.is_in_dm_group(2, 2).unwrap());
        assert!(DB.is_in_dm_group(3, 2).unwrap());
        DB.remove_dm_group(dm_group1).unwrap();
        assert!(!DB.is_in_dm_group(1, 1).unwrap());
        assert!(!DB.is_in_dm_group(2, 1).unwrap());
        assert!(!DB.is_in_dm_group(3, 1).unwrap());
        assert!(!DB.is_in_dm_group(1, 2).unwrap());
        assert!(DB.is_in_dm_group(2, 2).unwrap());
        assert!(DB.is_in_dm_group(3, 2).unwrap());
        DB.remove_dm_group(dm_group2).unwrap();
    }
}
