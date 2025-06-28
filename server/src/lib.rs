#[cfg(feature = "server")]
mod secret;

use std::{fmt::Display, str::FromStr};

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeDelta, Utc};
#[cfg(feature = "server")]
use dioxus::logger::tracing::{debug, error, info};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use shared::limits::LIMITS;
use shared::types::GroupPermissions;

#[cfg(feature = "server")]
use crate::secret::db::DB;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ServerError {
    CreateAccountDatabaseError,
    CreateSessionDatabaseError,
    FindUserDatabaseError,
    InvalidSessionToken,
    SendMessageDatabaseError,
    VerificationDatabaseError,
    Forbidden,
    FetchMessagesDatabaseError,
    InviteDatabaseError,
    GroupPartiallyCreated(u64),
    InvalidArgumentSize,
    InvalidValue,
    InvalidUserId,
    GroupDatabaseError,
    LimitExceeded,
    SignatureEarly,
    SignatureExpired,
    InvalidSignature,
    UnsupportedCryptographicAlgorithm,
    AccountNotFound,
    LoginAccountDatabaseError,
    GetUserDataDatabaseError,
    AlreadyInGroup,
}

impl FromStr for ServerError {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CreateAccountDatabaseError" => Ok(Self::CreateAccountDatabaseError),
            "CreateSessionDatabaseError" => Ok(Self::CreateSessionDatabaseError),
            "FindUserDatabaseError" => Ok(Self::FindUserDatabaseError),
            "InvalidSessionToken" => Ok(Self::InvalidSessionToken),
            "SendMessageDatabaseError" => Ok(Self::SendMessageDatabaseError),
            "VerificationDatabaseError" => Ok(Self::VerificationDatabaseError),
            "Forbidden" => Ok(Self::Forbidden),
            "FetchMessagesDatabaseError" => Ok(Self::FetchMessagesDatabaseError),
            "InviteDatabaseError" => Ok(Self::InviteDatabaseError),
            "InvalidArgumentSize" => Ok(Self::InvalidArgumentSize),
            "InvalidValue" => Ok(Self::InvalidValue),
            "InvalidUserId" => Ok(Self::InvalidUserId),
            "GroupDatabaseError" => Ok(Self::GroupDatabaseError),
            "LimitExceeded" => Ok(Self::LimitExceeded),
            "SignatureEarly" => Ok(Self::SignatureEarly),
            "SignatureExpired" => Ok(Self::SignatureExpired),
            "InvalidSignature" => Ok(Self::InvalidSignature),
            "UnsupportedCryptographicAlgorithm" => Ok(Self::UnsupportedCryptographicAlgorithm),
            "AccountNotFound" => Ok(Self::AccountNotFound),
            "LoginAccountDatabaseError" => Ok(Self::LoginAccountDatabaseError),
            "GetUserDataDatabaseError" => Ok(Self::GetUserDataDatabaseError),
            "AlreadyInGroup" => Ok(Self::AlreadyInGroup),
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
            Self::CreateAccountDatabaseError => "CreateAccountDatabaseError".to_owned(),
            Self::CreateSessionDatabaseError => "CreateSessionDatabaseError".to_owned(),
            Self::FindUserDatabaseError => "FindUserDatabaseError".to_owned(),
            Self::InvalidSessionToken => "InvalidSessionToken".to_owned(),
            Self::SendMessageDatabaseError => "SendMessageDatabaseError".to_owned(),
            Self::VerificationDatabaseError => "VerificationDatabaseError".to_owned(),
            Self::Forbidden => "Forbidden".to_owned(),
            Self::FetchMessagesDatabaseError => "FetchMessagesDatabaseError".to_owned(),
            Self::InviteDatabaseError => "InviteDatabaseError".to_owned(),
            Self::GroupPartiallyCreated(id) => format!("GroupPartiallyCreated:{id}"),
            Self::InvalidArgumentSize => "InvalidArgumentSize".to_owned(),
            Self::InvalidValue => "InvalidValue".to_owned(),
            Self::InvalidUserId => "InvalidUserId".to_owned(),
            Self::GroupDatabaseError => "GroupDatabaseError".to_owned(),
            Self::LimitExceeded => "LimitExceeded".to_owned(),
            Self::SignatureEarly => "SignatureEarly".to_owned(),
            Self::SignatureExpired => "SignatureExpired".to_owned(),
            Self::InvalidSignature => "InvalidSignature".to_owned(),
            Self::UnsupportedCryptographicAlgorithm => {
                "UnsupportedCryptographicAlgorithm".to_owned()
            }
            Self::AccountNotFound => "AccountNotFound".to_owned(),
            Self::LoginAccountDatabaseError => "LoginAccountDatabaseError".to_owned(),
            Self::GetUserDataDatabaseError => "GetUserDataDatabaseError".to_owned(),
            Self::AlreadyInGroup => "AlreadyInGroup".to_owned(),
        })?;
        Ok(())
    }
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: u64,
    pub public_key: Box<[u8]>,
    pub encrypted_private_info: Box<[u8]>,
    pub email: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserAccount {
    pub public_key: Box<[u8]>,
    pub email: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundAccount {
    pub id: u64,
    pub public_key: Box<[u8]>,
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmMessage {
    pub id: u64,
    pub encryption_method: String,
    pub content: Box<[u8]>,
    pub reply_to: Option<u64>,
    pub edit_for: Option<u64>,
    pub sent_time: Option<chrono::NaiveDateTime>,
    pub sent_by_me: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupMessage {
    pub id: u64,
    pub encryption_method: String,
    pub content: Box<[u8]>,
    pub reply_to: Option<u64>,
    pub edit_for: Option<u64>,
    pub sent_time: Option<chrono::NaiveDateTime>,
    pub sender_id: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountCredentials {
    pub id: u64,
    pub session_token: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmInvite {
    pub id: u64,
    pub initiator_id: u64,
    pub other_id: u64,
    pub encrypted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupInvite {
    pub id: u64,
    pub inviter_id: u64,
    pub invited_id: u64,
    pub group_id: u64,
    pub permissions: Box<[u8]>,
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
    pub icon: Box<[u8]>,
    pub encrypted: bool,
    pub public: bool,
    pub channel: bool,
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

#[server]
pub async fn create_account(
    email: String,
    username: String,
    public_key: Box<[u8]>,
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
                        ServerError::CreateSessionDatabaseError,
                    ))
                }
            }
        }
        Err(err) => {
            error!("Failed to create account: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::CreateAccountDatabaseError,
            ))
        }
    }
}

#[server]
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

    let Some(result) = shared::crypto::verify(&login_algorithm, &public_key, data, &signature)
    else {
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
                ServerError::LoginAccountDatabaseError,
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
                ServerError::CreateSessionDatabaseError,
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

#[server]
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

#[server]
pub async fn find_user(
    query: String,
    credentials: AccountCredentials,
) -> Result<Vec<FoundAccount>, ServerFnError<ServerError>> {
    if query.is_empty() {
        return Err(ServerFnError::WrappedServerError(ServerError::InvalidArgumentSize));
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
                ServerError::FindUserDatabaseError,
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
                ServerError::VerificationDatabaseError,
            ))
        }
    }
}

#[server]
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
                ServerError::SendMessageDatabaseError,
            ))
        }
    }
}

#[server]
pub async fn fetch_new_dm_messages(
    group_id: u64,
    last_received_message_id: u64,
    credentials: AccountCredentials,
) -> Result<Vec<DmMessage>, ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_in_dm_group(credentials.id, group_id)?;

    match DB.get_dm_messages(last_received_message_id, group_id, credentials.id) {
        Ok(messages) => Ok(messages),
        Err(err) => {
            error!("Failed to fetch new DM messages: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::FetchMessagesDatabaseError,
            ))
        }
    }
}

#[server]
pub async fn send_dm_invite(
    other_id: u64,
    encrypted: bool,
    credentials: AccountCredentials,
) -> Result<u64, ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_user(other_id)?;

    if credentials.id == other_id {
        return Err(ServerFnError::WrappedServerError(ServerError::InvalidValue));
    }

    match DB.add_dm_invite(credentials.id, other_id, encrypted) {
        Ok(id) => Ok(id),
        Err(err) => {
            error!("Failed to send DM invite: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InviteDatabaseError,
            ))
        }
    }
}

#[server]
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
                ServerError::InviteDatabaseError,
            ));
        }
    };

    if invite.other_id != credentials.id {
        return Err(ServerFnError::WrappedServerError(ServerError::Forbidden));
    }

    let group_id = match DB.create_dm_group(invite.initiator_id, invite.other_id, invite.encrypted)
    {
        Ok(id) => id,
        Err(err) => {
            error!("Failed to create DM group while trying to accept invite: {err:?}");
            return Err(ServerFnError::WrappedServerError(
                ServerError::InviteDatabaseError,
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

#[server]
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
                ServerError::InviteDatabaseError,
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
                ServerError::InviteDatabaseError,
            ))
        }
    }
}

#[server]
pub async fn get_sent_dm_invites(
    credentials: AccountCredentials,
) -> Result<Vec<DmInvite>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    match DB.get_sent_dm_invites(credentials.id) {
        Ok(invites) => Ok(invites),
        Err(err) => {
            error!("Failed to get sent DM invites: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InviteDatabaseError,
            ))
        }
    }
}

#[server]
pub async fn get_received_dm_invites(
    credentials: AccountCredentials,
) -> Result<Vec<DmInvite>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    match DB.get_received_dm_invites(credentials.id) {
        Ok(invites) => Ok(invites),
        Err(err) => {
            error!("Failed to get received DM invites: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::InviteDatabaseError,
            ))
        }
    }
}

#[server]
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
                ServerError::InviteDatabaseError,
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
                ServerError::InviteDatabaseError,
            ))
        }
    }
}

#[server]
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
                ServerError::GroupDatabaseError,
            ))
        }
    }
}

#[server]
pub async fn get_user_data(
    user_id: u64,
    credentials: AccountCredentials,
) -> Result<Option<UserAccount>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    match DB.get_user_by_id(user_id) {
        Ok(Some(account)) => Ok(Some(UserAccount {
            public_key: account.public_key,
            email: account.email,
            username: account.username,
        })),
        Ok(None) => Ok(None),
        Err(err) => {
            eprintln!("Failed to get user by id {user_id}: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::GetUserDataDatabaseError,
            ))
        }
    }
}

#[server]
pub async fn get_joined_dm_groups(
    credentials: AccountCredentials,
) -> Result<Vec<DmGroup>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    match DB.get_dm_groups(credentials.id) {
        Ok(groups) => Ok(groups),
        Err(err) => {
            error!("Failed to get joined DM groups of user {}: {err:?}", credentials.id);
            Err(ServerFnError::WrappedServerError(
                    ServerError::GroupDatabaseError,
            ))
        }
    }
}

#[server]
pub async fn get_joined_groups(
    credentials: AccountCredentials,
) -> Result<Vec<MultiUserGroup>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    match DB.get_groups(credentials.id) {
        Ok(groups) => Ok(groups),
        Err(err) => {
            error!("Failed to get joined multi-user groups of user {}: {err:?}", credentials.id);
            Err(ServerFnError::WrappedServerError(
                    ServerError::GroupDatabaseError,
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
                ServerError::VerificationDatabaseError,
            ))
        }
    }
}

#[cfg(feature = "server")]
pub fn check_is_not_in_group(user_id: u64, group_id: u64) -> Result<(), ServerFnError<ServerError>> {
    match DB.is_in_group(user_id, group_id) {
        Ok(value) => {
            if value {
                Err(ServerFnError::WrappedServerError(ServerError::AlreadyInGroup))
            } else {
                Ok(())
            }
        }
        Err(err) => {
            error!("Failed to check whether the user is in group or not: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::VerificationDatabaseError,
            ))
        }
    }
}

#[server]
pub async fn send_group_invite(
    user_id: u64,
    group_id: u64,
    permissions: Box<[u8]>,
    credentials: AccountCredentials,
) -> Result<u64, ServerFnError<ServerError>> {
    check_session(credentials)?;
    check_is_in_group(credentials.id, group_id)?;
    check_is_not_in_group(user_id, group_id)?;

    match DB.add_group_invite(credentials.id, user_id, group_id, &permissions) {
        Ok(invite_id) => Ok(invite_id),
        Err(err) => {
            error!("Failed to send group invite to user {user_id}: {err:?}");
            Err(ServerFnError::WrappedServerError(
                ServerError::GroupDatabaseError,
            ))
        }
    }
}

#[server]
pub async fn create_group(
    name: String,
    icon: Option<Box<[u8]>>,
    encrypted: bool,
    public: bool,
    channel: bool,
    credentials: AccountCredentials,
) -> Result<u64, ServerFnError<ServerError>> {
    check_session(credentials)?;

    let group_id = match DB.create_group(&name, &icon.unwrap_or(Box::new([])), encrypted, public, channel) {
        Ok(group_id) => group_id,
        Err(err) => {
            error!("Failed to create a new group: {err:?}");
            return Err(ServerFnError::WrappedServerError(ServerError::GroupDatabaseError));
        }
    };

    match DB.add_group_member(group_id, credentials.id, &GroupPermissions::admin().to_bytes()) {
        Ok(()) => Ok(group_id),
        Err(err) => {
            error!("Failed to add user creator to its group: {err:?}");
            Err(ServerFnError::WrappedServerError(ServerError::GroupPartiallyCreated(group_id)))
        }
    }
}

#[server]
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
                ServerError::FetchMessagesDatabaseError,
            ))
        }
    }
}

#[server]
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
                ServerError::SendMessageDatabaseError,
            ))
        }
    }
}

#[cfg(feature = "server")]
pub fn init_server() {
    println!("Initializing server");

    if let Err(err) = DB.init() {
        eprintln!("An error was encountered while initializing database: {err:?}");
    } else {
        println!("Database initialized successfully");
    }

    println!("Server initialized");
}
