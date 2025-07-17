use crate::{
    Account, DmGroup, DmInvite, DmMessage, GroupInvite, GroupMember, GroupMessage, MessageStatus,
    MultiUserGroup,
};
use shared::limits::LIMITS;
use shared::{crypto::x3dh::X3DhReceiverKeysPublic, types::GroupPermissions};

use std::sync::{Arc, LazyLock, Mutex};

use mysql::prelude::*;
use mysql::{Pool, Row, params};
use postcard::{from_bytes, to_allocvec};
use rand::{SeedableRng, rngs::StdRng};

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

    pub fn init(&self) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        conn.query_drop(
            r"
            CREATE TABLE IF NOT EXISTS `accounts` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `public_key` BLOB NOT NULL,
                `public_x3dh_data` BLOB NOT NULL,
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
                `permissions` BLOB NOT NULL
            );
        ",
        )?;
        conn.query_drop(format!(
            r"
            CREATE TABLE IF NOT EXISTS `dm_messages` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `sender_id` BIGINT NOT NULL,
                `group_id` BIGINT NOT NULL,
                `encryption_method` VARCHAR({}) NOT NULL,
                `reply_message_id` BIGINT,
                `edited_message_id` BIGINT,
                `content` BLOB NOT NULL,
                `send_time` DATETIME NOT NULL,
                `delivered` BIT NOT NULL
            );
        ",
            LIMITS.max_encryption_method_length
        ))?;
        conn.query_drop(format!(
            r"
            CREATE TABLE IF NOT EXISTS `group_messages` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `sender_id` BIGINT NOT NULL,
                `group_id` BIGINT NOT NULL,
                `encryption_method` VARCHAR({}) NOT NULL,
                `reply_message_id` BIGINT,
                `edited_message_id` BIGINT,
                `content` BLOB NOT NULL,
                `send_time` DATETIME NOT NULL
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
                `encryption_data` BLOB
            );
        ",
        )?;
        conn.query_drop(
            r"
            CREATE TABLE IF NOT EXISTS `group_invites` (
                `id` BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
                `inviter_id` BIGINT NOT NULL,
                `invited_id` BIGINT NOT NULL,
                `group_id` BIGINT NOT NULL,
                `permissions` VARCHAR(255) NOT NULL,
                `encryption_data` BLOB
            );
        ",
        )?;
        conn.query_drop(
            r"
            ALTER TABLE `sessions`
                ADD INDEX `session_token_idx` (`session_token`(32));
            ALTER TABLE `sessions`
                ADD INDEX `account_id_idx` (`account_id`);

            ALTER TABLE `group_members`
                ADD INDEX `user_groups_idx` (`user_id`, `group_id`),
                ADD INDEX `group_users_idx` (`group_id`, `user_id`);

            ALTER TABLE `group_messages`
                ADD INDEX `group_time_idx` (`group_id`, `send_time`);
        ",
        )?;
        Ok(())
    }

    pub fn create_account(
        &self,
        public_key: &[u8],
        public_x3dh_data: X3DhReceiverKeysPublic,
        encrypted_private_info: &[u8],
        email: Option<&str>,
        username: Option<&str>,
    ) -> DbResult<u64> {
        let mut conn = self.pool.get_conn()?;
        let public_x3dh_data = to_allocvec(&public_x3dh_data)?;
        if let Err(err) = from_bytes::<X3DhReceiverKeysPublic>(&public_x3dh_data) {
            eprintln!("From bytes failed for public X3DH data: {err:?}");
        };
        conn.exec_drop(
            r"INSERT INTO `accounts` (
                `public_key`,
                `public_x3dh_data`,
                `encrypted_private_info`,
                `email`,
                `username`
            ) VALUES (?, ?, ?, ?, ?);",
            (
                public_key,
                public_x3dh_data,
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
            ) VALUES (
                ?,
                ?,
                IFNULL(?, CURRENT_TIMESTAMP()),
                IFNULL(?, DATE_ADD(NOW(), INTERVAL 7 DAY))
            );",
            (account_id, session_token, begin_time, end_time),
        )?;
        Ok(session_token)
    }

    pub fn find_user(&self, query: &str, ignore_user: u64) -> DbResult<Vec<Account>> {
        let mut conn = self.pool.get_conn()?;
        let mut accounts = vec![];
        conn.exec_map(
            r"SELECT * FROM `accounts`
                WHERE (`username` LIKE CONCAT('%', :query, '%')
                    OR `email` LIKE CONCAT('%', :query, '%'))
                    AND `id` != :ignore_user
                LIMIT 10;",
            params! {
                query,
                ignore_user,
            },
            |(id, public_key, cryptoidentity, encrypted_private_info, email, username)| {
                if let Ok(cryptoidentity) = from_bytes(&cryptoidentity as &Box<[u8]>) {
                    accounts.push(Account {
                        id,
                        cryptoidentity,
                        public_key,
                        encrypted_private_info,
                        email,
                        username,
                    })
                }
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
            r"INSERT INTO `dm_messages` (
                `group_id`,
                `sender_id`,
                `encryption_method`,
                `reply_message_id`,
                `edited_message_id`,
                `content`,
                `send_time`,
                `delivered`
            ) VALUES (?, ?, ?, NULL, NULL, ?, IFNULL(?, CURRENT_TIMESTAMP()), 0)",
            (group_id, sender_id, encryption_method, content, send_time),
        )?;
        Ok(conn.query_first("SELECT LAST_INSERT_ID();")?.unwrap())
    }

    pub fn get_dm_messages(
        &self,
        last_message_id: u64,
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
                `send_time`,
                `delivered`
                FROM `dm_messages`
                WHERE `id` > ?
                    AND `group_id` = ?
                ORDER BY `send_time` DESC
                LIMIT 30;",
            (last_message_id, group_id),
            |(
                id,
                sender_id,
                encryption_method,
                reply_message_id,
                edited_message_id,
                content,
                send_time,
                delivered_bytes,
            )| {
                let _: u64 = sender_id;
                let _: Box<[u8]> = delivered_bytes;
                let delivered = delivered_bytes[0] != 0;
                DmMessage {
                    id,
                    encryption_method,
                    content,
                    reply_to: reply_message_id,
                    edit_for: edited_message_id,
                    sent_time: send_time,
                    status: if sender_id != account_id {
                        MessageStatus::SentByOther
                    } else if delivered {
                        MessageStatus::Delivered
                    } else {
                        MessageStatus::Sent
                    },
                }
            },
        )?;
        Ok(value)
    }

    pub fn add_dm_invite(
        &self,
        initiator_id: u64,
        other_id: u64,
        encryption_data: Option<&[u8]>,
    ) -> DbResult<u64> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"INSERT INTO `dm_invites` (
            `initiator_id`,
            `other_id`,
            `encryption_data`
        ) VALUES (?, ?, ?);",
            (initiator_id, other_id, encryption_data),
        )?;
        Ok(conn.query_first("SELECT LAST_INSERT_ID();")?.unwrap())
    }

    pub fn get_dm_invite(&self, id: u64) -> DbResult<DmInvite> {
        let mut conn = self.pool.get_conn()?;
        let mut invite: Row = conn
            .exec_first(
                r"SELECT * FROM `dm_invites`
            WHERE `id` = ?;",
                (id,),
            )?
            .unwrap();
        Ok(DmInvite {
            id: invite.take_opt(0).unwrap()?,
            initiator_id: invite.take_opt(1).unwrap()?,
            other_id: invite.take_opt(2).unwrap()?,
            encryption_data: if let Some(data) = invite.take_opt(3) {
                Some(data?)
            } else {
                None
            },
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
            |(id, initiator_id, other_id, encryption_data)| DmInvite {
                id,
                initiator_id,
                other_id,
                encryption_data,
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
            |(id, initiator_id, other_id, encryption_data)| DmInvite {
                id,
                initiator_id,
                other_id,
                encryption_data,
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

    pub fn find_user_with_pubkey(&self, account_name: String, public_key: &[u8]) -> DbResult<Option<u64>> {
        if account_name.len() >= 256 {
            return Ok(None);
        };
        let mut conn = self.pool.get_conn()?;
        let account: Option<u64> = conn.exec_first(
            r"SELECT `id` FROM `accounts`
            WHERE (`username` = ?
                OR `email` = ?)
                AND `public_key` = ?;",
            (account_name.clone(), account_name, public_key),
        )?;
        Ok(account)
    }

    pub fn get_user_by_id(&self, account_id: u64) -> DbResult<Option<Account>> {
        let mut conn = self.pool.get_conn()?;
        let Some(mut user) = conn.exec_first(
            r"SELECT * FROM `accounts`
            WHERE `id` = ?;",
            (account_id,),
        )?
        else {
            return Ok(None);
        };
        let _: Row = user;
        let cryptoidentity: Box<[u8]> = user.take_opt(2).unwrap()?;
        let cryptoidentity = from_bytes(&cryptoidentity)?;
        Ok(Some(Account {
            id: user.take_opt(0).unwrap()?,
            cryptoidentity,
            public_key: user.take_opt(1).unwrap()?,
            encrypted_private_info: user.take_opt(3).unwrap()?,
            email: user.take_opt(4).unwrap()?,
            username: user.take_opt(5).unwrap()?,
        }))
    }

    pub fn get_dm_groups(&self, account_id: u64) -> DbResult<Vec<DmGroup>> {
        let mut conn = self.pool.get_conn()?;
        let value = conn.exec_map(
            r"SELECT
                `id`,
                `encrypted`,
                `initiator_id`,
                `other_id`
                FROM `dm_groups`
                WHERE `initiator_id` = ?
                    OR `other_id` = ?
                ORDER BY `id` DESC
                LIMIT 30;",
            (account_id, account_id),
            |(id, encrypted_bytes, initiator_id, other_id)| {
                let _: Box<[u8]> = encrypted_bytes;
                DmGroup {
                    id,
                    encrypted: encrypted_bytes[0] != 0,
                    initiator_id,
                    other_id,
                }
            },
        )?;
        Ok(value)
    }

    pub fn create_group(
        &self,
        name: &str,
        encrypted: bool,
        public: bool,
        channel: bool,
    ) -> DbResult<u64> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"INSERT INTO `groups` (`name`, `encrypted`, `public`, `channel`)
                VALUES (?, ?, ?, ?);",
            (name, encrypted, public, channel),
        )?;
        // `LAST_INSERT_ID()` returns the last id only for the current Pool connection.
        let group_id: u64 = conn.query_first("SELECT LAST_INSERT_ID();")?.unwrap();
        Ok(group_id)
    }

    pub fn is_in_group(&self, sender_id: u64, group_id: u64) -> DbResult<bool> {
        let mut conn = self.pool.get_conn()?;
        let value: Option<u8> = conn.exec_first(
            r"SELECT 1 FROM `group_members`
                WHERE `user_id` = :sender_id
                    AND `group_id` = :group_id;",
            params! {
                group_id,
                sender_id,
            },
        )?;
        Ok(value.is_some())
    }

    pub fn send_group_message(
        &self,
        sender_id: u64,
        group_id: u64,
        encryption_method: &str,
        content: &[u8],
        send_time: Option<chrono::NaiveDateTime>,
    ) -> DbResult<u64> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"INSERT INTO `group_messages` (
                `group_id`,
                `sender_id`,
                `encryption_method`,
                `reply_message_id`,
                `edited_message_id`,
                `content`,
                `send_time`
            ) VALUES (?, ?, ?, NULL, NULL, ?, IFNULL(?, CURRENT_TIMESTAMP()))",
            (group_id, sender_id, encryption_method, content, send_time),
        )?;
        Ok(conn.query_first("SELECT LAST_INSERT_ID();")?.unwrap())
    }

    pub fn get_group_messages(
        &self,
        last_message_id: u64,
        group_id: u64,
    ) -> DbResult<Vec<GroupMessage>> {
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
                FROM `group_messages`
                WHERE `id` > ?
                    AND `group_id` = ?
                ORDER BY `send_time` DESC
                LIMIT 30;",
            (last_message_id, group_id),
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
                GroupMessage {
                    id,
                    sender_id,
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

    pub fn add_group_invite(
        &self,
        inviter_id: u64,
        invited_id: u64,
        group_id: u64,
        permissions: &[u8],
        encryption_data: Option<&[u8]>,
    ) -> DbResult<u64> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"INSERT INTO `group_invites` (
            `inviter_id`,
            `invited_id`,
            `group_id`,
            `permissions`,
            `encryption_data`
        ) VALUES (?, ?, ?, ?, ?);",
            (
                inviter_id,
                invited_id,
                group_id,
                permissions,
                encryption_data,
            ),
        )?;
        Ok(conn.query_first("SELECT LAST_INSERT_ID();")?.unwrap())
    }

    pub fn get_group_invite(&self, id: u64) -> DbResult<GroupInvite> {
        let mut conn = self.pool.get_conn()?;
        let mut invite: Row = conn
            .exec_first(
                r"SELECT * FROM `group_invites`
            WHERE `id` = ?;",
                (id,),
            )?
            .unwrap();
        Ok(GroupInvite {
            id: invite.take_opt(0).unwrap()?,
            inviter_id: invite.take_opt(1).unwrap()?,
            invited_id: invite.take_opt(2).unwrap()?,
            group_id: invite.take_opt(3).unwrap()?,
            permissions: invite.take_opt(4).unwrap()?,
            encryption_data: if let Some(data) = invite.take_opt(5) {
                Some(data?)
            } else {
                None
            },
        })
    }

    pub fn remove_group_invite(&self, id: u64) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"DELETE FROM `group_invites`
            WHERE `id` = ?;",
            (id,),
        )?;
        Ok(())
    }

    pub fn get_sent_group_invites(&self, id: u64) -> DbResult<Vec<GroupInvite>> {
        let mut conn = self.pool.get_conn()?;
        let value = conn.exec_map(
            r"SELECT
                *
                FROM `group_invites`
                WHERE `inviter_id` = ? 
                ORDER BY `id` DESC
                LIMIT 30;",
            (id,),
            |(id, inviter_id, invited_id, group_id, permissions, encryption_data)| GroupInvite {
                id,
                inviter_id,
                invited_id,
                group_id,
                permissions,
                encryption_data,
            },
        )?;
        Ok(value)
    }

    pub fn get_received_group_invites(&self, id: u64) -> DbResult<Vec<GroupInvite>> {
        let mut conn = self.pool.get_conn()?;
        let value = conn.exec_map(
            r"SELECT
                *
                FROM `group_invites`
                WHERE `invited_id` = ? 
                ORDER BY `id` DESC
                LIMIT 30;",
            (id,),
            |(id, inviter_id, invited_id, group_id, permissions, encryption_data)| GroupInvite {
                id,
                inviter_id,
                invited_id,
                group_id,
                permissions,
                encryption_data,
            },
        )?;
        Ok(value)
    }

    pub fn remove_group(&self, group_id: u64) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        Ok(conn.exec_drop(
            r"DELETE FROM `groups`
            WHERE id = ?",
            (group_id,),
        )?)
    }

    pub fn get_group_ids(&self, account_id: u64) -> DbResult<Vec<u64>> {
        let mut conn = self.pool.get_conn()?;
        let group_ids: Vec<u64> = conn.exec_map(
            r"SELECT
                `group_id`
                FROM `group_members`
                WHERE `user_id` = ?
                ORDER BY `group_id` DESC
                LIMIT 30;",
            (account_id,),
            |group_id| group_id,
        )?;
        Ok(group_ids)
    }

    pub fn get_group_by_id(&self, group_id: u64) -> DbResult<Option<MultiUserGroup>> {
        let mut conn = self.pool.get_conn()?;
        let Some(mut group) = conn.exec_first(
            r"SELECT
                *
                FROM `groups`
                WHERE `id` = ?;",
            (group_id,),
        )?
        else {
            return Ok(None);
        };
        let _: Row = group;
        let encrypted_bytes: Box<[u8]> = group.take_opt(2).unwrap()?;
        let public_bytes: Box<[u8]> = group.take_opt(3).unwrap()?;
        let channel_bytes: Box<[u8]> = group.take_opt(4).unwrap()?;
        Ok(Some(MultiUserGroup {
            id: group.take_opt(0).unwrap()?,
            name: group.take_opt(1).unwrap()?,
            icon: None,
            encrypted: encrypted_bytes[0] != 0,
            public: public_bytes[0] != 0,
            channel: channel_bytes[0] != 0,
        }))
    }

    pub fn get_groups(&self, account_id: u64) -> DbResult<Vec<MultiUserGroup>> {
        let group_ids = self.get_group_ids(account_id)?;
        let mut groups = vec![];
        groups.reserve_exact(group_ids.len());

        for id in group_ids {
            if let Some(group) = self.get_group_by_id(id)? {
                groups.push(group);
            }
        }

        Ok(groups)
    }

    pub fn add_group_member(
        &self,
        group_id: u64,
        user_id: u64,
        permissions: &[u8],
    ) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"INSERT INTO `group_members` (
            `group_id`,
            `user_id`,
            `permissions`
        ) VALUES (?, ?, ?);",
            (group_id, user_id, permissions),
        )?;
        Ok(())
    }

    pub fn get_group_member_count(&self, group_id: u64) -> DbResult<Option<u64>> {
        let mut conn = self.pool.get_conn()?;
        let value = conn.exec_first(
            r"SELECT COUNT(*) FROM `group_members`
            WHERE `group_id` = ?;",
            (group_id,),
        )?;
        Ok(value)
    }

    pub fn get_group_members(&self, group_id: u64) -> DbResult<Vec<GroupMember>> {
        let mut conn = self.pool.get_conn()?;
        let value: Vec<GroupMember> = conn.exec_map(
            r"SELECT `user_id`, `permissions` FROM `group_members`
            WHERE `group_id` = ?;",
            (group_id,),
            |(user_id, permissions)| {
                let _: Box<[u8]> = permissions;
                GroupMember {
                    user_id,
                    is_admin: GroupPermissions::from_bytes(&permissions).is_admin(),
                }
            },
        )?;
        Ok(value)
    }

    pub fn remove_group_member(&self, group_id: u64, user_id: u64) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"DELETE FROM `group_members`
            WHERE `group_id` = ?
                AND `user_id` = ?;",
            (group_id, user_id),
        )?;
        Ok(())
    }

    pub fn set_group_member_permissions(
        &self,
        group_id: u64,
        user_id: u64,
        permissions: GroupPermissions,
    ) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"UPDATE `group_members`
            SET `permissions` = ?
            WHERE `group_id` = ?
                AND `user_id` = ?;",
            (permissions.to_bytes(), group_id, user_id),
        )?;
        Ok(())
    }

    pub fn mark_dm_message_delivered(&self, group_id: u64, message_id: u64) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        conn.exec_drop(
            r"UPDATE `dm_messages`
            SET `delivered` = 1
            WHERE `group_id` = ?
                AND `id` = ?;",
            (group_id, message_id),
        )?;
        Ok(())
    }

    pub fn get_group_member_permissions(
        &self,
        group_id: u64,
        user_id: u64,
    ) -> DbResult<Option<GroupPermissions>> {
        let mut conn = self.pool.get_conn()?;
        let Some(permission_bytes) = conn.exec_first(
            r"SELECT `permissions`
            FROM `group_members`
            WHERE `group_id` = ?
                AND `user_id` = ?;",
            (group_id, user_id),
        )?
        else {
            return Ok(None);
        };
        let _: Box<[u8]> = permission_bytes;
        Ok(Some(GroupPermissions::from_bytes(&permission_bytes)))
    }

    pub fn reset(&self) -> DbResult<()> {
        let mut conn = self.pool.get_conn()?;
        conn.query_drop("DROP TABLE IF EXISTS `accounts`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `sessions`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `groups`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `dm_groups`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `group_members`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `dm_messages`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `group_messages`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `read_messages`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `dm_invites`;")?;
        conn.query_drop("DROP TABLE IF EXISTS `group_invites`;")?;
        self.init()?;
        Ok(())
    }
}

static RNG: LazyLock<Arc<Mutex<StdRng>>> =
    LazyLock::new(|| Arc::new(Mutex::new(StdRng::from_os_rng())));
pub static DB: LazyLock<Database> =
    LazyLock::new(|| Database::try_new(&std::env::var("DB_URL").unwrap()).unwrap());

// TODO: Move into another module
pub mod rng {
    use super::RNG;
    use rand::RngCore;

    pub fn fill_bytes(destination: &mut [u8]) {
        RNG.lock().unwrap().fill_bytes(destination);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{LazyLock, Mutex, Once},
    };

    use crate::{DmInvite, MessageStatus, secret::db::Account};

    use super::Database;
    use shared::crypto::{x3dh::{self, X3DhReceiverKeysPublic}, preferred_alogirthm};

    static DB: LazyLock<Database> =
        LazyLock::new(|| Database::try_new(&std::env::var("TEST_DB_URL").unwrap()).unwrap());
    static INIT: Once = Once::new();
    static DB_TEST_NUMBER: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
    static CRYPTOIDENTITIES: LazyLock<Mutex<HashMap<u64, X3DhReceiverKeysPublic>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    fn db_test(test_number: usize, test_fn: fn()) {
        INIT.call_once(|| {
            DB.reset().unwrap();
        });
        let mut test_lock;
        loop {
            test_lock = DB_TEST_NUMBER.lock().unwrap();
            if *test_lock == test_number {
                break;
            }
            drop(test_lock);
        }
        test_fn();
        *test_lock = test_number + 1;
    }

    fn cryptoidentity_for(user_id: u64) -> X3DhReceiverKeysPublic {
        if let Some(cryptoidentity) = CRYPTOIDENTITIES.lock().unwrap().get(&user_id) {
            cryptoidentity.clone()
        } else {
            let (_, cryptoidentity) = x3dh::generate_receiver_keys(&preferred_alogirthm()).unwrap();
            CRYPTOIDENTITIES
                .lock()
                .unwrap()
                .insert(user_id, cryptoidentity.clone());
            cryptoidentity
        }
    }

    #[test]
    fn create_accounts() {
        db_test(0, || {
            for id in 0..=6 {
                assert!(!DB.is_valid_user_id(id).unwrap());
            }
            DB.create_account(
                &[1],
                cryptoidentity_for(1),
                &[],
                Some("some_email@example.com"),
                Some("The first User"),
            )
            .unwrap();
            assert!(!DB.is_valid_user_id(0).unwrap());
            assert!(DB.is_valid_user_id(1).unwrap());
            assert!(!DB.is_valid_user_id(2).unwrap());
            DB.create_account(
                &[2],
                cryptoidentity_for(2),
                &[],
                None,
                Some("The second user"),
            )
            .unwrap();
            assert!(!DB.is_valid_user_id(0).unwrap());
            assert!(DB.is_valid_user_id(1).unwrap());
            assert!(DB.is_valid_user_id(2).unwrap());
            assert!(!DB.is_valid_user_id(3).unwrap());
            DB.create_account(
                &[3],
                cryptoidentity_for(3),
                &[],
                Some("third_user@example.com"),
                None,
            )
            .unwrap();
            assert!(!DB.is_valid_user_id(0).unwrap());
            assert!(DB.is_valid_user_id(1).unwrap());
            assert!(DB.is_valid_user_id(2).unwrap());
            assert!(DB.is_valid_user_id(3).unwrap());
            assert!(!DB.is_valid_user_id(4).unwrap());
            assert!(DB.get_user_by_id(4).unwrap().is_none());
            DB.create_account(&[4], cryptoidentity_for(4), &[], None, None)
                .unwrap();
            assert_eq!(DB.get_user_by_id(4).unwrap().unwrap().id, 4);
            assert!(!DB.is_valid_user_id(0).unwrap());
            assert!(DB.is_valid_user_id(1).unwrap());
            assert!(DB.is_valid_user_id(2).unwrap());
            assert!(DB.is_valid_user_id(3).unwrap());
            assert!(DB.is_valid_user_id(4).unwrap());
            assert!(!DB.is_valid_user_id(5).unwrap());
            DB.create_account(
                &[5],
                cryptoidentity_for(5),
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
        });
    }

    #[test]
    fn test_find_accounts() {
        db_test(1, || {
            assert_eq!(
                DB.find_user("user", 0).unwrap(),
                vec![
                    Account {
                        id: 1,
                        cryptoidentity: cryptoidentity_for(1),
                        public_key: Box::new([1]),
                        encrypted_private_info: Box::new([]),
                        email: Some("some_email@example.com".to_owned()),
                        username: Some("The first User".to_owned()),
                    },
                    Account {
                        id: 2,
                        cryptoidentity: cryptoidentity_for(2),
                        public_key: Box::new([2]),
                        encrypted_private_info: Box::new([]),
                        email: None,
                        username: Some("The second user".to_owned()),
                    },
                    Account {
                        id: 3,
                        cryptoidentity: cryptoidentity_for(3),
                        public_key: Box::new([3]),
                        encrypted_private_info: Box::new([]),
                        email: Some("third_user@example.com".to_owned()),
                        username: None,
                    },
                ],
            );
            assert_eq!(
                DB.find_user("user", 2).unwrap(),
                vec![
                    Account {
                        id: 1,
                        cryptoidentity: cryptoidentity_for(1),
                        public_key: Box::new([1]),
                        encrypted_private_info: Box::new([]),
                        email: Some("some_email@example.com".to_owned()),
                        username: Some("The first User".to_owned()),
                    },
                    Account {
                        id: 3,
                        cryptoidentity: cryptoidentity_for(3),
                        public_key: Box::new([3]),
                        encrypted_private_info: Box::new([]),
                        email: Some("third_user@example.com".to_owned()),
                        username: None,
                    },
                ],
            );
        });
    }

    #[test]
    fn create_sessions() {
        db_test(2, || {
            let token = DB.create_session(1, None, None).unwrap();
            assert!(DB.is_session_valid(1, token).unwrap());
            assert!(!DB.is_session_valid(2, token).unwrap());
            assert!(!DB.is_session_valid(3, token).unwrap());
            let token2 = DB.create_session(2, None, None).unwrap();
            assert!(!DB.is_session_valid(1, token2).unwrap());
            assert!(DB.is_session_valid(2, token2).unwrap());
            assert!(!DB.is_session_valid(3, token2).unwrap());
        });
    }

    #[test]
    fn test_invites() {
        db_test(3, || {
            let invite1 = DmInvite {
                id: 1,
                initiator_id: 1,
                other_id: 2,
                encryption_data: None,
            };
            let invite2 = DmInvite {
                id: 2,
                initiator_id: 3,
                other_id: 2,
                encryption_data: None,
            };
            let invite3 = DmInvite {
                id: 3,
                initiator_id: 3,
                other_id: 1,
                encryption_data: None,
            };
            DB.add_dm_invite(
                invite1.initiator_id,
                invite1.other_id,
                invite1.encryption_data.as_deref(),
            )
            .unwrap();
            DB.add_dm_invite(
                invite2.initiator_id,
                invite2.other_id,
                invite2.encryption_data.as_deref(),
            )
            .unwrap();
            DB.add_dm_invite(
                invite3.initiator_id,
                invite3.other_id,
                invite3.encryption_data.as_deref(),
            )
            .unwrap();
            assert_eq!(DB.get_sent_dm_invites(1).unwrap(), vec![invite1.clone()]);
            assert_eq!(
                DB.get_received_dm_invites(1).unwrap(),
                vec![invite3.clone()]
            );
            assert_eq!(DB.get_sent_dm_invites(2).unwrap(), vec![]);
            assert_eq!(
                DB.get_received_dm_invites(2).unwrap(),
                vec![invite2.clone(), invite1.clone()]
            );
            assert_eq!(
                DB.get_sent_dm_invites(3).unwrap(),
                vec![invite3, invite2.clone()]
            );
            assert_eq!(DB.get_received_dm_invites(3).unwrap(), vec![]);
            DB.remove_dm_invite(3).unwrap();
            assert_eq!(DB.get_sent_dm_invites(1).unwrap(), vec![invite1.clone()]);
            assert_eq!(DB.get_received_dm_invites(1).unwrap(), vec![]);
            assert_eq!(DB.get_sent_dm_invites(2).unwrap(), vec![]);
            assert_eq!(
                DB.get_received_dm_invites(2).unwrap(),
                vec![invite2.clone(), invite1]
            );
            assert_eq!(DB.get_sent_dm_invites(3).unwrap(), vec![invite2]);
            assert_eq!(DB.get_received_dm_invites(3).unwrap(), vec![]);
        });
    }

    #[test]
    fn create_dm_groups() {
        db_test(4, || {
            assert!(!DB.is_in_dm_group(1, 1).unwrap());
            assert!(!DB.is_in_dm_group(2, 1).unwrap());
            assert!(!DB.is_in_dm_group(3, 1).unwrap());
            assert!(!DB.is_in_dm_group(1, 2).unwrap());
            assert!(!DB.is_in_dm_group(2, 2).unwrap());
            assert!(!DB.is_in_dm_group(3, 2).unwrap());
            assert!(DB.get_dm_groups(1).unwrap().is_empty());
            assert!(DB.get_dm_groups(2).unwrap().is_empty());
            assert!(DB.get_dm_groups(3).unwrap().is_empty());
            let dm_group1 = DB.create_dm_group(1, 2, true).unwrap();
            assert_eq!(DB.get_dm_groups(1).unwrap().len(), 1);
            assert_eq!(DB.get_dm_groups(2).unwrap().len(), 1);
            assert!(DB.get_dm_groups(3).unwrap().is_empty());
            assert!(DB.is_in_dm_group(1, 1).unwrap());
            assert!(DB.is_in_dm_group(2, 1).unwrap());
            assert!(!DB.is_in_dm_group(3, 1).unwrap());
            assert!(!DB.is_in_dm_group(1, 2).unwrap());
            assert!(!DB.is_in_dm_group(2, 2).unwrap());
            assert!(!DB.is_in_dm_group(3, 2).unwrap());
            assert_eq!(dm_group1, 1);
        });
    }

    #[test]
    fn send_dm_messages() {
        db_test(5, || {
            let dm_group1 = 1;

            DB.send_dm_message(1, dm_group1, "!plaintext", "Hello, World!".as_bytes(), None)
                .unwrap();
            DB.send_dm_message(2, dm_group1, "privatecipher123", &[0x69, 0x68], None)
                .unwrap();
            DB.mark_dm_message_delivered(dm_group1, 1).unwrap();
            let dm_messages1 = DB.get_dm_messages(0, dm_group1, 1).unwrap();
            assert_eq!(dm_messages1[0].id, 1);
            assert_eq!(dm_messages1[0].encryption_method, "!plaintext");
            assert_eq!(dm_messages1[0].content, "Hello, World!".as_bytes().into());
            assert_eq!(dm_messages1[0].reply_to, None);
            assert_eq!(dm_messages1[0].edit_for, None);
            assert_eq!(dm_messages1[0].status, MessageStatus::Delivered);
            assert_eq!(dm_messages1[1].id, 2);
            assert_eq!(dm_messages1[1].encryption_method, "privatecipher123");
            assert_eq!(dm_messages1[1].content, [0x69, 0x68].into());
            assert_eq!(dm_messages1[1].reply_to, None);
            assert_eq!(dm_messages1[1].edit_for, None);
            assert_eq!(dm_messages1[1].status, MessageStatus::SentByOther);
            assert_eq!(dm_messages1.len(), 2);
            let mut dm_messages2 = DB.get_dm_messages(0, dm_group1, 2).unwrap();
            dm_messages2[0].status = match dm_messages2[0].status {
                MessageStatus::SentByOther => MessageStatus::Delivered,
                _ => panic!(),
            };
            dm_messages2[1].status = match dm_messages2[1].status {
                MessageStatus::Sent => MessageStatus::SentByOther,
                _ => panic!(),
            };
            assert_eq!(dm_messages1, dm_messages2);
            dm_messages2[0].status = MessageStatus::SentByOther;
            dm_messages2[1].status = MessageStatus::Sent;
            let dm_messages3 = DB.get_dm_messages(1, dm_group1, 2).unwrap();
            assert_eq!(dm_messages2[1], dm_messages3[0]);
            assert_eq!(dm_messages3.len(), 1);
        });
    }

    #[test]
    fn test_dm_groups() {
        db_test(6, || {
            let dm_group1 = 1;

            assert_eq!(DB.get_dm_groups(1).unwrap().len(), 1);
            assert_eq!(DB.get_dm_groups(2).unwrap().len(), 1);
            assert!(DB.get_dm_groups(3).unwrap().is_empty());
            assert!(DB.get_dm_groups(4).unwrap().is_empty());
            let dm_group2 = DB.create_dm_group(3, 2, true).unwrap();
            assert_eq!(DB.get_dm_groups(1).unwrap().len(), 1);
            assert_eq!(DB.get_dm_groups(2).unwrap().len(), 2);
            assert_eq!(DB.get_dm_groups(3).unwrap().len(), 1);
            assert!(DB.get_dm_groups(4).unwrap().is_empty());
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
        });
    }

    #[test]
    fn create_groups() {
        db_test(7, || {
            assert!(DB.get_groups(1).unwrap().is_empty());
            assert!(DB.get_groups(2).unwrap().is_empty());
            assert!(DB.get_groups(3).unwrap().is_empty());
            assert!(DB.get_groups(4).unwrap().is_empty());
            let group1 = DB
                .create_group("Some public group", false, true, false)
                .unwrap();
            assert!(DB.get_groups(1).unwrap().is_empty());
            assert_eq!(group1, 1);
            DB.add_group_member(group1, 1, &[0xFF]).unwrap();
            assert_eq!(DB.get_groups(1).unwrap().len(), 1);
            assert!(DB.get_groups(2).unwrap().is_empty());
            assert!(DB.get_groups(3).unwrap().is_empty());
            assert!(DB.get_groups(4).unwrap().is_empty());
        });
    }
}
