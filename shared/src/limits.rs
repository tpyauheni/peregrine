pub struct Limits {
    // Account registration/login limits
    pub max_username_length: usize,
    pub max_email_length: usize,
    pub max_public_key_length: usize,
    pub max_session_before_period: u32,
    pub max_session_after_period: u32,
    pub max_session_validity_period: u32,

    pub max_encryption_method_length: usize,
    pub max_message_length: usize,
}

pub static LIMITS: Limits = Limits {
    max_username_length: 32,
    max_email_length: 254,
    max_public_key_length: 16 * 1024,
    max_session_before_period: 3 * 24 * 60 * 60,
    max_session_after_period: 7 * 24 * 60 * 60,
    max_session_validity_period: 365 * 24 * 60 * 60,

    max_encryption_method_length: 16,
    max_message_length: 16 * 1024,
};
