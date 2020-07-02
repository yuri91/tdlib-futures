use futures::StreamExt;

use crate::types::*;
use crate::methods::*;
use crate::client::{Receiver, Sender};

pub enum Credentials {
    User {
        phone: String,
        getcode: Box<dyn FnMut()->String>,
    },
    Bot {
        token: String,
    }
}
pub struct AuthParameters {
    tdlib: TdlibParameters,
    encryption_key: String,
    credentials: Credentials
}
impl AuthParameters {
    pub fn for_user<T: 'static + FnMut()->String>(tdlib: TdlibParameters, encryption_key: String, phone: String, getcode: T) -> AuthParameters {
        AuthParameters {
            tdlib,
            encryption_key,
            credentials: Credentials::User {
                phone,
                getcode: Box::new(getcode),
            },
        }
    }
    pub fn for_bot(tdlib: TdlibParameters, encryption_key: String, token: String) -> AuthParameters {
        AuthParameters {
            tdlib,
            encryption_key,
            credentials: Credentials::Bot {
                token
            },
        }
    }
}

macro_rules! wait_for_authorization_state {
    ($receiver:expr, $name:ident) => {
        loop {
            let update = $receiver.next().await.expect("no update");
            if let Update::UpdateAuthorizationState(state) = update {
                if let AuthorizationState::AuthorizationStateReady(_a) = state.authorization_state {
                    return Ok(());
                }
                else if let AuthorizationState::$name(_a) = state.authorization_state {
                    break;
                } else {
                    return Err(Error{code:-1, message: format!("unexpected state: {:?}", state)});
                }
            }
        }
    }
}
pub async fn authorize(params: AuthParameters, sender: &mut Sender, receiver: &mut Receiver) -> Result<(), Error> {
    wait_for_authorization_state!(receiver, AuthorizationStateWaitTdlibParameters);
    let s = SetTdlibParameters {
        parameters: params.tdlib
    };
    sender.send(s).await?;
    wait_for_authorization_state!(receiver, AuthorizationStateWaitEncryptionKey);
    let s = CheckDatabaseEncryptionKey {
        encryption_key: params.encryption_key,
    };
    sender.send(s).await?;
    wait_for_authorization_state!(receiver, AuthorizationStateWaitPhoneNumber);
    match params.credentials {
        Credentials::User { phone, mut getcode } => {
            let s = SetAuthenticationPhoneNumber {
                phone_number: phone,
                settings: PhoneNumberAuthenticationSettings {
                    allow_flash_call: false,
                    is_current_phone_number: false,
                    allow_sms_retriever_api: false,
                },
            };
            sender.send(s).await?;
            wait_for_authorization_state!(receiver, AuthorizationStateWaitCode);
            let s = CheckAuthenticationCode {
                code: (getcode)(),
            };
            sender.send(s).await?;
        },
        Credentials::Bot { token } => {
            let s = CheckAuthenticationBotToken {
                token,
            };
            sender.send(s).await?;
        }
    }
    wait_for_authorization_state!(receiver, AuthorizationStateReady);
    return Ok(());
}
