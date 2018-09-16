use ::serde::de::DeserializeOwned;
use ::serde::Serialize;

#[derive(Deserialize, Debug)]
#[serde(tag = "@type")]
#[serde(rename_all = "camelCase")]
pub enum Update {
    UpdateAuthorizationState {
        authorization_state: UpdateAuthorizationState,
    },
    UpdateOption,
    UpdateConnectionState,
    UpdateUser,
    UpdateUserStatus,
    UpdateNotificationSettings,
}
#[derive(Deserialize, Debug)]
#[serde(tag = "@type")]
#[serde(rename_all = "camelCase")]
pub enum UpdateAuthorizationState {
    AuthorizationStateWaitTdlibParameters,
    AuthorizationStateWaitEncryptionKey,
    AuthorizationStateWaitPhoneNumber,
    AuthorizationStateWaitCode,
    AuthorizationStateWaitPassword,
    AuthorizationStateReady,
}


#[derive(Deserialize, Debug, Clone)]
pub struct Message {
    pub id: i64,
    pub sender_user_id: i32,
    pub chat_id: i64,
    pub is_outgoing: bool,
    pub can_be_edited: bool,
    pub can_be_forwarded: bool,
    pub can_be_deleted_only_for_self: bool,
    pub can_be_deleted_for_all_users: bool,
    pub is_channel_post: bool,
    pub contains_unread_mention: bool,
    pub date: i32,
    pub edit_date: i32,
    //pub forward_info,
    pub reply_to_message_id: i64,
    pub ttl: i32,
    pub ttl_expires_in: f64,
    pub via_bot_user_id: i32,
    pub author_signature: String,
    pub views: i32,
    pub media_album_id: String,
    pub content: MessageContent,
    //pub reply_markup
}
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "@type")]
#[serde(rename_all = "camelCase")]
pub enum MessageContent {
    MessageText(MessageText),
}

#[derive(Deserialize, Debug, Clone)]
pub struct MessageText {
    pub text: FormattedText,
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "@type")]
#[serde(rename_all = "camelCase")]
pub enum InputMessageContent {
    InputMessageText(InputMessageText),
}

#[derive(Serialize, Debug, Clone)]
pub struct InputMessageText {
    pub text: FormattedText,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormattedText {
    pub text: String,
}

pub trait Method: Serialize+Clone {
    const TYPE: &'static str;
    type Response: DeserializeOwned;

    fn tag(self) -> MethodType<Self>
        where Self: ::std::marker::Sized {
        MethodType {
            type_: Self::TYPE,
            payload: self,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct MethodType<T: Method> {
    #[serde(rename="@type")]
    pub type_: &'static str,
    #[serde(flatten)]
    pub payload: T,
}

#[derive(Serialize, Debug, Clone)]
pub struct SendMessage {
    pub chat_id: i64,
    pub reply_to_message_id: i64,
    pub disable_notification: bool,
    pub from_background: bool,
    pub input_message_content: InputMessageContent,
}

impl Method for SendMessage {
    const TYPE: &'static str = "sendMessage";
    type Response = Message;
}

#[derive(Deserialize, Debug)]
#[serde(tag="@type")]
#[serde(rename_all="snake_case")]
pub enum OK {
    Ok,
}


#[derive(Serialize, Debug, Clone)]
pub struct SetTdlibParameters {
    pub parameters: TdlibParameters,
}
#[derive(Serialize, Debug, Clone)]
pub struct TdlibParameters {
    pub use_test_dc: bool,
    pub api_id: i64,
    pub api_hash: String,
    pub device_model: String,
    pub system_version: String,
    pub application_version: String,
    pub system_language_code: String,
    pub files_directory: String,
    pub use_chat_info_database: bool,
    pub use_message_database: bool,
}

impl Method for SetTdlibParameters {
    const TYPE: &'static str = "setTdlibParameters";
    type Response = OK;
}

#[derive(Serialize, Debug, Clone)]
pub struct CheckDatabaseEncryptionKey {
    pub encryption_key: String,
}

impl Method for CheckDatabaseEncryptionKey {
    const TYPE: &'static str = "checkDatabaseEncryptionKey";
    type Response = OK;
}

#[derive(Serialize, Debug, Clone)]
pub struct SetAuthenticationPhoneNumber {
    pub phone_number: String,
    pub allow_flash_call: bool,
    pub is_current_phone_number: bool,
}

impl Method for SetAuthenticationPhoneNumber {
    const TYPE: &'static str = "setAuthenticationPhoneNumber";
    type Response = OK;
}


#[derive(Serialize, Debug, Clone)]
pub struct CheckAuthenticationCode {
    pub code: String,
}

impl Method for CheckAuthenticationCode {
    const TYPE: &'static str = "checkAuthenticationCode";
    type Response = OK;
}
