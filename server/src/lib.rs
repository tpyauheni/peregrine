#[cfg(feature = "server")]
pub mod secret;

use std::{fmt::Display, str::FromStr};

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use chrono::NaiveDateTime;
#[cfg(feature = "server")]
use chrono::{DateTime, TimeDelta, Utc};
#[cfg(feature = "server")]
use dioxus::logger::tracing::{debug, error, info};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use shared::crypto::PublicKey;
#[cfg(feature = "server")]
use shared::limits::LIMITS;
#[cfg(feature = "server")]
use shared::types::GroupPermissions;
use shared::{crypto::x3dh::X3DhReceiverKeysPublic, types::UserIcon};

#[cfg(feature = "server")]
use crate::secret::db::DB;
#[cfg(feature = "server")]
use crate::secret::storage::STORAGE;
#[cfg(feature = "server")]
use shared::storage::{GeneralStorage, RawStorage};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ServerError {
    InternalDatabaseError,
    InvalidSessionToken,
    Forbidden,
    GroupPartiallyCreated(u64),
    InvalidArgumentSize,
    InvalidValue,
    InvalidUserId,
    LimitExceeded,
    SignatureEarly,
    SignatureExpired,
    InvalidSignature,
    UnsupportedCryptographicAlgorithm,
    AccountNotFound,
    AlreadyInGroup,
    GroupPartiallyJoined,
    InvalidGroupId,
    ActionOnSelfIsForbidden,
}

impl FromStr for ServerError {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "InternalDatabaseError" => Ok(Self::InternalDatabaseError),
            "InvalidSessionToken" => Ok(Self::InvalidSessionToken),
            "Forbidden" => Ok(Self::Forbidden),
            "InvalidArgumentSize" => Ok(Self::InvalidArgumentSize),
            "InvalidValue" => Ok(Self::InvalidValue),
            "InvalidUserId" => Ok(Self::InvalidUserId),
            "LimitExceeded" => Ok(Self::LimitExceeded),
            "SignatureEarly" => Ok(Self::SignatureEarly),
            "SignatureExpired" => Ok(Self::SignatureExpired),
            "InvalidSignature" => Ok(Self::InvalidSignature),
            "UnsupportedCryptographicAlgorithm" => Ok(Self::UnsupportedCryptographicAlgorithm),
            "AccountNotFound" => Ok(Self::AccountNotFound),
            "AlreadyInGroup" => Ok(Self::AlreadyInGroup),
            "GroupPartiallyJoined" => Ok(Self::GroupPartiallyJoined),
            "InvalidGroupId" => Ok(Self::InvalidGroupId),
            "ActionOnSelfIsForbidden" => Ok(Self::ActionOnSelfIsForbidden),
            _ => {
                let Some(s_split) = s.split_once(':') else {
                    return Err(());
                };
                if s_split.0 == "GroupPartiallyCreated" {
                    let Ok(id) = s_split.1.parse::<u64>() else {
                        return Err(());
                    };
                    Ok(Self::GroupPartiallyCreated(id))
                } else {
                    Err(())
                }
            }
        }
    }
}

impl Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match *self {
            Self::InternalDatabaseError => "InternalDatabaseError".to_owned(),
            Self::InvalidSessionToken => "InvalidSessionToken".to_owned(),
            Self::Forbidden => "Forbidden".to_owned(),
            Self::GroupPartiallyCreated(id) => format!("GroupPartiallyCreated:{id}"),
            Self::InvalidArgumentSize => "InvalidArgumentSize".to_owned(),
            Self::InvalidValue => "InvalidValue".to_owned(),
            Self::InvalidUserId => "InvalidUserId".to_owned(),
            Self::LimitExceeded => "LimitExceeded".to_owned(),
            Self::SignatureEarly => "SignatureEarly".to_owned(),
            Self::SignatureExpired => "SignatureExpired".to_owned(),
            Self::InvalidSignature => "InvalidSignature".to_owned(),
            Self::UnsupportedCryptographicAlgorithm => {
                "UnsupportedCryptographicAlgorithm".to_owned()
            }
            Self::AccountNotFound => "AccountNotFound".to_owned(),
            Self::AlreadyInGroup => "AlreadyInGroup".to_owned(),
            Self::GroupPartiallyJoined => "GroupPartiallyJoined".to_owned(),
            Self::InvalidGroupId => "InvalidGroupId".to_owned(),
            Self::ActionOnSelfIsForbidden => "ActionOnSelfIsForbidden".to_owned(),
        })?;
        Ok(())
    }
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: u64,
    pub cryptoidentity: X3DhReceiverKeysPublic,
    pub public_key: Box<[u8]>,
    pub encrypted_private_info: Box<[u8]>,
    pub email: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserAccount {
    pub cryptoidentity: X3DhReceiverKeysPublic,
    pub public_key: Box<[u8]>,
    pub email: Option<String>,
    pub username: Option<String>,
    pub icon: UserIcon,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundAccount {
    pub id: u64,
    pub cryptoidentity: X3DhReceiverKeysPublic,
    pub public_key: Box<[u8]>,
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageStatus {
    SentByOther,
    Sent,
    Delivered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmMessage {
    pub id: u64,
    pub encryption_method: String,
    pub content: Box<[u8]>,
    pub reply_to: Option<u64>,
    pub edit_for: Option<u64>,
    pub sent_time: Option<NaiveDateTime>,
    pub status: MessageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupMessage {
    pub id: u64,
    pub encryption_method: String,
    pub content: Box<[u8]>,
    pub reply_to: Option<u64>,
    pub edit_for: Option<u64>,
    pub sent_time: Option<NaiveDateTime>,
    pub sender_id: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountCredentials {
    pub id: u64,
    pub session_token: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmInvite {
    pub id: u64,
    pub initiator_id: u64,
    pub other_id: u64,
    pub encryption_data: Option<Box<[u8]>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInvite {
    pub id: u64,
    pub inviter_id: u64,
    pub invited_id: u64,
    pub group_id: u64,
    pub permissions: Box<[u8]>,
    pub encryption_data: Option<Box<[u8]>>,
}

/// Describes parameters of a requested session.
/// `current_timestamp` is the current time in seconds since Unix epoch;
/// Signature of a session request is considered valid if timestamp in server is in range
/// `[current_timestamp - authorize_before_seconds; current_timestamp + authorize_after_seconds]`.
/// If it is valid and no errors occur, server issues session token which is valid until
/// `current_timestamp + session_validity_seconds`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionParams {
    pub current_timestamp: u64,
    pub authorize_before_seconds: u32,
    pub authorize_after_seconds: u32,
    pub session_validity_seconds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmGroup {
    pub id: u64,
    pub encrypted: bool,
    pub initiator_id: u64,
    pub other_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiUserGroup {
    pub id: u64,
    pub name: String,
    pub icon: UserIcon,
    pub encrypted: bool,
    pub public: bool,
    pub channel: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupMember {
    pub user_id: u64,
    pub is_admin: bool,
}

impl FromStr for AccountCredentials {
    type Err = usize;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = BASE64_URL_SAFE_NO_PAD.decode(s).unwrap_or_default();
        if bytes.len() != 40 {
            return Err(bytes.len());
        }
        let id = u64::from_le_bytes(bytes[..8].try_into().unwrap());
        let session_token: [u8; 32] = bytes[8..].try_into().unwrap();
        Ok(Self { id, session_token })
    }
}

impl Display for AccountCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut bytes = vec![];
        bytes.reserve_exact(40);
        bytes.extend(self.id.to_le_bytes());
        bytes.extend(self.session_token);
        f.write_str(&BASE64_URL_SAFE_NO_PAD.encode(bytes))?;
        Ok(())
    }
}

impl SessionParams {
    pub fn to_boxed_slice(&self) -> Box<[u8]> {
        let mut result: Vec<u8> = vec![];
        result.extend(self.current_timestamp.to_le_bytes());
        result.extend(self.authorize_before_seconds.to_le_bytes());
        result.extend(self.authorize_after_seconds.to_le_bytes());
        result.extend(self.session_validity_seconds.to_le_bytes());
        result.into_boxed_slice()
    }
}

#[server(endpoint = "create_account")]
pub async fn create_account(
    email: String,
    username: String,
    public_key: Box<[u8]>,
    cryptoidentity: X3DhReceiverKeysPublic,
) -> Result<(u64, [u8; 32]), ServerFnError<ServerError>> {
    if email.len() > LIMITS.max_email_length
        || public_key.len() > LIMITS.max_public_key_length
        || username.len() > LIMITS.max_username_length
    {
        return Err(ServerFnError::WrappedServerError(
            ServerError::InvalidArgumentSize,
        ));
    }

    match DB.create_account(
        &public_key,
        cryptoidentity,
        &[],
        Some(&email),
        if username.is_empty() {
            None
        } else {
            Some(&username)
        },
    ) {
        Ok(account_id) => {
            info!("New account created: {account_id}");
            match DB.create_session(account_id, None, None) {
                Ok(session_id) => {
                    debug!("New session created: {session_id:?}");
                    Ok((account_id, session_id))
                }
                Err(err) => {
                    error!("Failed to create session: {err:?}");
                    Err(ServerFnError::WrappedServerError(
                        ServerError::InternalDatabaseError,
                    ))
                }
            }
        }
        Err(err) => {
            error!("Failed to create account: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "login_account")]
pub async fn login_account(
    id: u64,
    login_algorithm: String,
    public_key: Box<[u8]>,
    session_params: SessionParams,
    signature: Box<[u8]>,
) -> Result<[u8; 32], ServerFnError<ServerError>> {
    if session_params.authorize_before_seconds >= LIMITS.max_session_before_period
        || session_params.authorize_after_seconds >= LIMITS.max_session_after_period
        || session_params.session_validity_seconds >= LIMITS.max_session_validity_period
    {
        return Err(ServerFnError::WrappedServerError(
            ServerError::LimitExceeded,
        ));
    }
    let current_time = Utc::now();
    let Some(expiration_seconds) =
        TimeDelta::try_seconds(session_params.session_validity_seconds as i64)
    else {
        return Err(ServerFnError::WrappedServerError(
            ServerError::LimitExceeded,
        ));
    };
    let Some(expiration_time) = current_time.checked_add_signed(expiration_seconds) else {
        return Err(ServerFnError::WrappedServerError(
            ServerError::LimitExceeded,
        ));
    };
    let unix_secs_now = current_time
        .signed_duration_since(DateTime::UNIX_EPOCH)
        .num_seconds()
        .cast_unsigned();

    if unix_secs_now
        < session_params.current_timestamp - session_params.authorize_before_seconds as u64
    {
        return Err(ServerFnError::WrappedServerError(
            ServerError::SignatureEarly,
        ));
    }
    if unix_secs_now
        > session_params.current_timestamp + session_params.authorize_after_seconds as u64
    {
        return Err(ServerFnError::WrappedServerError(
            ServerError::SignatureExpired,
        ));
    }

    let data = &session_params.to_boxed_slice();

    let Some(result) = shared::crypto::verify(
        &login_algorithm,
        PublicKey {
            pk: public_key.clone(),
        },
        data,
        &signature,
    ) else {
        return Err(ServerFnError::WrappedServerError(
            ServerError::UnsupportedCryptographicAlgorithm,
        ));
    };

    if !result {
        return Err(ServerFnError::WrappedServerError(
            ServerError::InvalidSignature,
        ));
    }

    match DB.has_user_pubkey(id, &public_key) {
        Ok(result) => {
            if !result {
                return Err(ServerFnError::WrappedServerError(
                    ServerError::AccountNotFound,
                ));
            }
        }
        Err(err) => {
            error!("Failed to check if user has pubkey while loggin into account: {err:?}");
            return Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ));
        }
    }

    match DB.create_session(
        id,
        Some(current_time.naive_utc()),
        Some(expiration_time.naive_utc()),
    ) {
        Ok(session_id) => {
            debug!("New session created: {session_id:?}");
            Ok(session_id)
        }
        Err(err) => {
            error!("Failed to create login session: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[cfg(feature = "server")]
fn check_session(credentials: AccountCredentials) -> Result<(), ServerFnError<ServerError>> {
    match secret::db::DB.is_session_valid(credentials.id, credentials.session_token) {
        Ok(is_valid) => {
            if is_valid {
                Ok(())
            } else {
                Err(ServerFnError::WrappedServerError(
                    ServerError::InvalidSessionToken,
                ))
            }
        }
        Err(err) => {
            error!("Failed to check if session is valid: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InvalidSessionToken,
            ))
        }
    }
}

#[server(endpoint = "are_session_credentials_valid")]
pub async fn are_session_credentials_valid(
    credentials: AccountCredentials,
) -> Result<bool, ServerFnError<ServerError>> {
    match check_session(credentials) {
        Ok(()) => Ok(true),
        Err(err) => {
            if err == ServerFnError::WrappedServerError(ServerError::InvalidSessionToken) {
                Ok(false)
            } else {
                Err(err)
            }
        }
    }
}

#[cfg(feature = "server")]
fn check_user(user_id: u64) -> Result<(), ServerFnError<ServerError>> {
    match secret::db::DB.is_valid_user_id(user_id) {
        Ok(is_valid) => {
            if is_valid {
                Ok(())
            } else {
                Err(ServerFnError::WrappedServerError(
                    ServerError::InvalidUserId,
                ))
            }
        }
        Err(err) => {
            error!("Failed to check if specified user exists: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InvalidUserId,
            ))
        }
    }
}

#[server(endpoint = "find_user")]
pub async fn find_user(
    query: String,
    credentials: AccountCredentials,
) -> Result<Vec<FoundAccount>, ServerFnError<ServerError>> {
    if query.is_empty() {
        return Err(ServerFnError::WrappedServerError(
            ServerError::InvalidArgumentSize,
        ));
    }

    if query.len() > LIMITS.max_email_length.max(LIMITS.max_username_length) {
        return Err(ServerFnError::WrappedServerError(
            ServerError::InvalidArgumentSize,
        ));
    }

    check_session(credentials)?;

    match DB.find_user(&query, credentials.id) {
        Ok(result) => {
            let mut found_accounts = vec![];
            found_accounts.reserve_exact(result.len());

            for account in result {
                found_accounts.push(FoundAccount {
                    id: account.id,
                    cryptoidentity: account.cryptoidentity,
                    public_key: account.public_key,
                    username: account.username,
                    email: account.email,
                });
            }

            Ok(found_accounts)
        }
        Err(err) => {
            error!("Failed to find user: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[cfg(feature = "server")]
pub fn check_is_in_dm_group(user_id: u64, group_id: u64) -> Result<(), ServerFnError<ServerError>> {
    match DB.is_in_dm_group(user_id, group_id) {
        Ok(value) => {
            if value {
                Ok(())
            } else {
                Err(ServerFnError::WrappedServerError(ServerError::Forbidden))
            }
        }
        Err(err) => {
            error!("Failed to check whether the user is in DM group or not: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "send_dm_message")]
pub async fn send_dm_message(
    group_id: u64,
    encryption_method: String,
    message: Box<[u8]>,
    credentials: AccountCredentials,
) -> Result<u64, ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_in_dm_group(credentials.id, group_id)?;

    if encryption_method.len() > LIMITS.max_encryption_method_length {
        return Err(ServerFnError::WrappedServerError(
            ServerError::InvalidArgumentSize,
        ));
    }

    if message.len() > LIMITS.max_message_length {
        return Err(ServerFnError::WrappedServerError(
            ServerError::InvalidArgumentSize,
        ));
    }

    match DB.send_dm_message(credentials.id, group_id, &encryption_method, &message, None) {
        Ok(id) => Ok(id),
        Err(err) => {
            error!("Failed to send DM message: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "fetch_new_dm_messages")]
pub async fn fetch_new_dm_messages(
    group_id: u64,
    last_received_message_id: u64,
    credentials: AccountCredentials,
) -> Result<Vec<DmMessage>, ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_in_dm_group(credentials.id, group_id)?;

    let result = match DB.get_dm_messages(last_received_message_id, group_id, credentials.id) {
        Ok(messages) => messages,
        Err(err) => {
            error!("Failed to fetch new DM messages: {err:?}");
            return Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ));
        }
    };

    for message in result.iter() {
        if message.status == MessageStatus::SentByOther {
            let db_result = DB.mark_dm_message_delivered(group_id, message.id);
            if let Err(err) = db_result {
                error!(
                    "Failed to mark DM message {} as delivered: {err:?}",
                    message.id
                );
            }
        }
    }

    Ok(result)
}

#[server(endpoint = "send_dm_invite")]
pub async fn send_dm_invite(
    other_id: u64,
    encryption_data: Option<Box<[u8]>>,
    credentials: AccountCredentials,
) -> Result<u64, ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_user(other_id)?;

    if credentials.id == other_id {
        return Err(ServerFnError::WrappedServerError(ServerError::InvalidValue));
    }

    match DB.add_dm_invite(credentials.id, other_id, encryption_data.as_deref()) {
        Ok(id) => Ok(id),
        Err(err) => {
            error!("Failed to send DM invite: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "accept_dm_invite")]
pub async fn accept_dm_invite(
    invite_id: u64,
    credentials: AccountCredentials,
) -> Result<u64, ServerFnError<ServerError>> {
    check_session(credentials)?;

    let invite = match DB.get_dm_invite(invite_id) {
        Ok(invite) => invite,
        Err(err) => {
            error!("Failed to get DM invite while trying to accept: {err:?}");
            return Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ));
        }
    };

    if invite.other_id != credentials.id {
        return Err(ServerFnError::WrappedServerError(ServerError::Forbidden));
    }

    let group_id = match DB.create_dm_group(
        invite.initiator_id,
        invite.other_id,
        invite.encryption_data.is_some(),
    ) {
        Ok(id) => id,
        Err(err) => {
            error!("Failed to create DM group while trying to accept invite: {err:?}");
            return Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ));
        }
    };

    match DB.remove_dm_invite(invite_id) {
        Ok(()) => Ok(group_id),
        Err(err) => {
            error!("Failed to accept DM invite (after creating group): {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::GroupPartiallyCreated(group_id),
            ))
        }
    }
}

#[server(endpoint = "reject_dm_invite")]
pub async fn reject_dm_invite(
    invite_id: u64,
    credentials: AccountCredentials,
) -> Result<(), ServerFnError<ServerError>> {
    check_session(credentials)?;

    let invite = match DB.get_dm_invite(invite_id) {
        Ok(invite) => invite,
        Err(err) => {
            error!("Failed to get DM invite while trying to reject: {err:?}");
            return Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ));
        }
    };

    if invite.other_id != credentials.id {
        return Err(ServerFnError::WrappedServerError(ServerError::Forbidden));
    }

    match DB.remove_dm_invite(invite_id) {
        Ok(()) => Ok(()),
        Err(err) => {
            error!("Failed to reject DM invite: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "get_sent_dm_invites")]
pub async fn get_sent_dm_invites(
    credentials: AccountCredentials,
) -> Result<Vec<DmInvite>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    match DB.get_sent_dm_invites(credentials.id) {
        Ok(invites) => Ok(invites),
        Err(err) => {
            error!("Failed to get sent DM invites: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "get_received_dm_invites")]
pub async fn get_received_dm_invites(
    credentials: AccountCredentials,
) -> Result<Vec<DmInvite>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    match DB.get_received_dm_invites(credentials.id) {
        Ok(invites) => Ok(invites),
        Err(err) => {
            error!("Failed to get received DM invites: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "cancel_dm_invite")]
pub async fn cancel_dm_invite(
    invite_id: u64,
    credentials: AccountCredentials,
) -> Result<(), ServerFnError<ServerError>> {
    check_session(credentials)?;

    let invite = match DB.get_dm_invite(invite_id) {
        Ok(invite) => invite,
        Err(err) => {
            error!("Failed to get DM invite while trying to reject: {err:?}");
            return Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ));
        }
    };

    if invite.initiator_id != credentials.id {
        return Err(ServerFnError::WrappedServerError(ServerError::Forbidden));
    }

    match DB.remove_dm_invite(invite_id) {
        Ok(()) => Ok(()),
        Err(err) => {
            error!("Failed to cancel DM invite: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "leave_dm_group")]
pub async fn leave_dm_group(
    group_id: u64,
    credentials: AccountCredentials,
) -> Result<(), ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_in_dm_group(credentials.id, group_id)?;

    match DB.remove_dm_group(group_id) {
        Ok(()) => Ok(()),
        Err(err) => {
            error!("Failed to leave DM group: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[cfg(feature = "server")]
fn store_icon(prefix: &str, id: u64, icon: Box<[u8]>) {
    STORAGE.store(&format!("{prefix}{id}.bin"), &icon);
}

#[cfg(feature = "server")]
fn load_icon(prefix: &str, id: u64) -> UserIcon {
    STORAGE.raw_load(format!("{prefix}{id}.bin")).ok()
}

#[server(endpoint = "get_user_data")]
pub async fn get_user_data(
    user_id: u64,
    credentials: AccountCredentials,
) -> Result<Option<UserAccount>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    let icon = load_icon("u", user_id);

    match DB.get_user_by_id(user_id) {
        Ok(Some(account)) => Ok(Some(UserAccount {
            public_key: account.public_key,
            cryptoidentity: account.cryptoidentity,
            email: account.email,
            username: account.username,
            icon,
        })),
        Ok(None) => Ok(None),
        Err(err) => {
            eprintln!("Failed to get user by id {user_id}: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "get_group_data")]
pub async fn get_group_data(
    group_id: u64,
    credentials: AccountCredentials,
) -> Result<Option<MultiUserGroup>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    let err = check_is_in_group(credentials.id, group_id);

    match DB.get_group_by_id(group_id) {
        Ok(Some(mut group)) => {
            if let Err(err) = err
                && !group.public
            {
                return Err(err);
            }

            let icon = load_icon("g", group_id);
            group.icon = icon;

            Ok(Some(group))
        }
        Ok(None) => Err(ServerFnError::WrappedServerError(ServerError::Forbidden)),
        Err(err) => {
            eprintln!("Failed to get group data by id {group_id}: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "get_joined_dm_groups")]
pub async fn get_joined_dm_groups(
    credentials: AccountCredentials,
) -> Result<Vec<DmGroup>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    match DB.get_dm_groups(credentials.id) {
        Ok(groups) => Ok(groups),
        Err(err) => {
            error!(
                "Failed to get joined DM groups of user {}: {err:?}",
                credentials.id
            );
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "get_joined_groups")]
pub async fn get_joined_groups(
    credentials: AccountCredentials,
) -> Result<Vec<MultiUserGroup>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    match DB.get_groups(credentials.id) {
        Ok(groups) => Ok(groups),
        Err(err) => {
            error!(
                "Failed to get joined multi-user groups of user {}: {err:?}",
                credentials.id
            );
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[cfg(feature = "server")]
pub fn check_is_in_group(user_id: u64, group_id: u64) -> Result<(), ServerFnError<ServerError>> {
    match DB.is_in_group(user_id, group_id) {
        Ok(value) => {
            if value {
                Ok(())
            } else {
                Err(ServerFnError::WrappedServerError(ServerError::Forbidden))
            }
        }
        Err(err) => {
            error!("Failed to check whether the user is in group or not: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[cfg(feature = "server")]
pub fn check_is_not_in_group(
    user_id: u64,
    group_id: u64,
) -> Result<(), ServerFnError<ServerError>> {
    match DB.is_in_group(user_id, group_id) {
        Ok(value) => {
            if value {
                Err(ServerFnError::WrappedServerError(
                    ServerError::AlreadyInGroup,
                ))
            } else {
                Ok(())
            }
        }
        Err(err) => {
            error!("Failed to check whether the user is in group or not: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[cfg(feature = "server")]
pub fn check_is_group_admin(group_id: u64, user_id: u64) -> Result<(), ServerFnError<ServerError>> {
    match DB.get_group_member_permissions(group_id, user_id) {
        Ok(Some(permissions)) => {
            if permissions.is_admin() {
                Ok(())
            } else {
                Err(ServerFnError::WrappedServerError(ServerError::Forbidden))
            }
        }
        Ok(None) => Err(ServerFnError::WrappedServerError(ServerError::Forbidden)),
        Err(err) => {
            error!("Failed to check whether the user is the group admin or not: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "send_group_invite")]
pub async fn send_group_invite(
    user_id: u64,
    group_id: u64,
    permissions: Box<[u8]>,
    credentials: AccountCredentials,
    encryption_data: Option<Box<[u8]>>,
) -> Result<u64, ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_in_group(credentials.id, group_id)?;
    check_is_not_in_group(user_id, group_id)?;

    match DB.add_group_invite(
        credentials.id,
        user_id,
        group_id,
        &permissions,
        encryption_data.as_deref(),
    ) {
        Ok(invite_id) => Ok(invite_id),
        Err(err) => {
            error!("Failed to send group invite to user {user_id}: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "create_group")]
pub async fn create_group(
    name: String,
    icon: Option<Box<[u8]>>,
    encrypted: bool,
    public: bool,
    channel: bool,
    credentials: AccountCredentials,
) -> Result<u64, ServerFnError<ServerError>> {
    check_session(credentials)?;

    if let Some(icon) = icon.as_ref()
        && icon.len() > LIMITS.max_group_icon_size
    {
        return Err(ServerFnError::WrappedServerError(
            ServerError::LimitExceeded,
        ));
    }

    let group_id = match DB.create_group(&name, encrypted, public, channel) {
        Ok(group_id) => group_id,
        Err(err) => {
            error!("Failed to create a new group: {err:?}");
            return Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ));
        }
    };

    if let Some(icon) = icon {
        store_icon("g", group_id, icon);
    }

    match DB.add_group_member(
        group_id,
        credentials.id,
        &GroupPermissions::admin().to_bytes(),
    ) {
        Ok(()) => Ok(group_id),
        Err(err) => {
            error!("Failed to add user creator to its group: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::GroupPartiallyCreated(group_id),
            ))
        }
    }
}

#[server(endpoint = "fetch_new_group_messages")]
pub async fn fetch_new_group_messages(
    group_id: u64,
    last_received_message_id: u64,
    credentials: AccountCredentials,
) -> Result<Vec<GroupMessage>, ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_in_group(credentials.id, group_id)?;

    match DB.get_group_messages(last_received_message_id, group_id) {
        Ok(messages) => Ok(messages),
        Err(err) => {
            error!("Failed to fetch new group messages: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "send_group_message")]
pub async fn send_group_message(
    group_id: u64,
    encryption_method: String,
    message: Box<[u8]>,
    credentials: AccountCredentials,
) -> Result<u64, ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_in_group(credentials.id, group_id)?;

    if encryption_method.len() > LIMITS.max_encryption_method_length {
        return Err(ServerFnError::WrappedServerError(
            ServerError::InvalidArgumentSize,
        ));
    }

    if message.len() > LIMITS.max_message_length {
        return Err(ServerFnError::WrappedServerError(
            ServerError::InvalidArgumentSize,
        ));
    }

    match DB.send_group_message(credentials.id, group_id, &encryption_method, &message, None) {
        Ok(id) => Ok(id),
        Err(err) => {
            error!("Failed to send group message: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "get_sent_group_invites")]
pub async fn get_sent_group_invites(
    credentials: AccountCredentials,
) -> Result<Vec<GroupInvite>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    match DB.get_sent_group_invites(credentials.id) {
        Ok(invites) => Ok(invites),
        Err(err) => {
            error!("Failed to get sent group invites: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "get_received_group_invites")]
pub async fn get_received_group_invites(
    credentials: AccountCredentials,
) -> Result<Vec<GroupInvite>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    match DB.get_received_group_invites(credentials.id) {
        Ok(invites) => Ok(invites),
        Err(err) => {
            error!("Failed to get received group invites: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "cancel_group_invite")]
pub async fn cancel_group_invite(
    invite_id: u64,
    credentials: AccountCredentials,
) -> Result<(), ServerFnError<ServerError>> {
    check_session(credentials)?;

    let invite = match DB.get_group_invite(invite_id) {
        Ok(invite) => invite,
        Err(err) => {
            error!("Failed to get group invite while trying to reject: {err:?}");
            return Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ));
        }
    };

    if invite.inviter_id != credentials.id {
        return Err(ServerFnError::WrappedServerError(ServerError::Forbidden));
    }

    match DB.remove_group_invite(invite_id) {
        Ok(()) => Ok(()),
        Err(err) => {
            error!("Failed to cancel group invite: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "accept_group_invite")]
pub async fn accept_group_invite(
    invite_id: u64,
    credentials: AccountCredentials,
) -> Result<(), ServerFnError<ServerError>> {
    check_session(credentials)?;

    let invite = match DB.get_group_invite(invite_id) {
        Ok(invite) => invite,
        Err(err) => {
            error!("Failed to get group invite while trying to accept: {err:?}");
            return Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ));
        }
    };

    if invite.invited_id != credentials.id {
        return Err(ServerFnError::WrappedServerError(ServerError::Forbidden));
    }

    match DB.add_group_member(
        invite.group_id,
        invite.invited_id,
        &GroupPermissions::default().to_bytes(),
    ) {
        Ok(id) => id,
        Err(err) => {
            error!("Failed to create group while trying to accept invite: {err:?}");
            return Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ));
        }
    };

    match DB.remove_group_invite(invite_id) {
        Ok(()) => Ok(()),
        Err(err) => {
            error!("Failed to accept group invite (after creating group): {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::GroupPartiallyJoined,
            ))
        }
    }
}

#[server(endpoint = "reject_group_invite")]
pub async fn reject_group_invite(
    invite_id: u64,
    credentials: AccountCredentials,
) -> Result<(), ServerFnError<ServerError>> {
    check_session(credentials)?;

    let invite = match DB.get_group_invite(invite_id) {
        Ok(invite) => invite,
        Err(err) => {
            error!("Failed to get group invite while trying to reject: {err:?}");
            return Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ));
        }
    };

    if invite.invited_id != credentials.id {
        return Err(ServerFnError::WrappedServerError(ServerError::Forbidden));
    }

    match DB.remove_group_invite(invite_id) {
        Ok(()) => Ok(()),
        Err(err) => {
            error!("Failed to reject group invite: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "get_group_member_count")]
pub async fn get_group_member_count(
    group_id: u64,
    credentials: AccountCredentials,
) -> Result<u64, ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_in_group(credentials.id, group_id)?;

    match DB.get_group_member_count(group_id) {
        Ok(Some(member_count)) => Ok(member_count),
        // In theory it's possible that `check_is_in_group` will return `Ok`-value then the group
        // will be removed and after that `DB.get_group_member_count` will be called.
        Ok(None) => Err(ServerFnError::WrappedServerError(
            ServerError::InvalidGroupId,
        )),
        Err(err) => {
            error!("Failed to get group member count: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "get_group_members")]
pub async fn get_group_members(
    group_id: u64,
    credentials: AccountCredentials,
) -> Result<Vec<GroupMember>, ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_in_group(credentials.id, group_id)?;

    match DB.get_group_members(group_id) {
        Ok(members) => Ok(members),
        Err(err) => {
            error!("Failed to get group members: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "kick_group_member")]
pub async fn kick_group_member(
    group_id: u64,
    user_id: u64,
    credentials: AccountCredentials,
) -> Result<(), ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_group_admin(group_id, credentials.id)?;

    if credentials.id == user_id {
        return Err(ServerFnError::WrappedServerError(
            ServerError::ActionOnSelfIsForbidden,
        ));
    }

    match DB.remove_group_member(group_id, user_id) {
        Ok(()) => Ok(()),
        Err(err) => {
            error!("Failed to kick user from a group: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "promote_group_member")]
pub async fn promote_group_member(
    group_id: u64,
    user_id: u64,
    credentials: AccountCredentials,
) -> Result<(), ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_group_admin(group_id, credentials.id)?;

    if credentials.id == user_id {
        return Err(ServerFnError::WrappedServerError(
            ServerError::ActionOnSelfIsForbidden,
        ));
    }

    match DB.set_group_member_permissions(group_id, user_id, GroupPermissions::admin()) {
        Ok(()) => Ok(()),
        Err(err) => {
            error!("Failed to promote user in a group: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "demote_group_member")]
pub async fn demote_group_member(
    group_id: u64,
    user_id: u64,
    credentials: AccountCredentials,
) -> Result<(), ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_group_admin(group_id, credentials.id)?;

    if credentials.id == user_id {
        return Err(ServerFnError::WrappedServerError(
            ServerError::ActionOnSelfIsForbidden,
        ));
    }

    match DB.set_group_member_permissions(group_id, user_id, GroupPermissions::default()) {
        Ok(()) => Ok(()),
        Err(err) => {
            error!("Failed to demote user in a group: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[server(endpoint = "leave_group")]
pub async fn leave_group(
    group_id: u64,
    credentials: AccountCredentials,
) -> Result<(), ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_in_group(credentials.id, group_id)?;

    match DB.remove_group_member(group_id, credentials.id) {
        Ok(()) => Ok(()),
        Err(err) => {
            error!("Failed to leave from a group: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InternalDatabaseError,
            ))
        }
    }
}

#[cfg(feature = "server")]
pub fn init_server() {
    println!("Initializing server");

    if std::env::var("PEREGRINE_RESET_DATABASE").unwrap_or("0".to_owned()) == "1" {
        println!("RESETTING DATABASE IN 10 SECONDS...");
        std::thread::sleep(std::time::Duration::from_secs(10));

        if let Err(err) = DB.reset() {
            eprintln!("An error was encountered while resetting database: {err:?}");
        } else {
            println!("Database resetted successfully");
        }
    } else if let Err(err) = DB.init() {
        eprintln!("An error was encountered while initializing database: {err:?}");
    } else {
        println!("Database initialized successfully");
    }

    println!("Server initialized");
}
