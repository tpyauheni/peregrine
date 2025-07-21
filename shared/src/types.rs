use serde::{Deserialize, Serialize};

// TODO: Really check for permissions.
pub struct GroupPermissions {
    pub send_messages: bool,
    pub read_messages: bool,
    pub invite_users: bool,

    pub custom_permissions: Vec<String>,
}

impl Default for GroupPermissions {
    fn default() -> Self {
        Self {
            send_messages: true,
            read_messages: true,
            invite_users: true,
            custom_permissions: vec![],
        }
    }
}

impl GroupPermissions {
    pub fn to_bytes(&self) -> Box<[u8]> {
        let mut general_permissions: u128 = 0;
        if self.send_messages {
            general_permissions |= 1;
        }
        if self.read_messages {
            general_permissions |= 2;
        }
        if self.invite_users {
            general_permissions |= 4;
        }
        let mut bytes = vec![];
        bytes.extend(general_permissions.to_le_bytes());

        for custom_permission in self.custom_permissions.iter() {
            let perm_bytes = custom_permission.as_bytes();
            assert!(perm_bytes.len() < 256);
            bytes.extend((perm_bytes.len() as u8).to_le_bytes());
            bytes.extend(perm_bytes);
        }

        bytes.into_boxed_slice()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let general_permissions: u128 = u128::from_le_bytes(bytes[..16].try_into().unwrap());
        let mut custom_permissions: Vec<String> = vec![];
        let mut index = 16;

        while index < bytes.len() {
            let length = bytes[index] as usize;
            index += 1;
            let permission_name = &bytes[index..index + length];
            custom_permissions.push(String::from_utf8_lossy(permission_name).to_string());
            index += length;
        }

        Self {
            send_messages: general_permissions & 1 != 0,
            read_messages: general_permissions & 2 != 0,
            invite_users: general_permissions & 4 != 0,
            custom_permissions,
        }
    }

    pub fn admin() -> Self {
        Self {
            send_messages: true,
            read_messages: true,
            invite_users: true,
            custom_permissions: vec!["admin".to_owned()],
        }
    }

    pub fn is_admin(&self) -> bool {
        self.custom_permissions.contains(&"admin".to_owned())
    }
}

pub type UserIcon = Option<Box<[u8]>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct File {
    pub name: Box<[u8]>,
    pub content: Box<[u8]>,
    pub encryption_method: String,
}
