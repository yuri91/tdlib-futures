use std::sync::Arc;
use futures::lock::Mutex;
use std::sync::atomic::AtomicUsize;
use std::collections::HashMap;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use log::error;
use futures::channel::{mpsc, oneshot};
use futures::{SinkExt, StreamExt};

use crate::types::*;
use crate::methods::*;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Request<T: Method> {
    #[serde(rename = "@extra")]
    id: usize,
    #[serde(flatten)]
    payload: MethodType<T>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
enum Message {
    Response(Response),
    Update(Update),
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Response {
    #[serde(rename = "@extra")]
    id: usize,
    #[serde(flatten)]
    payload: serde_json::Value,
}

pub fn init() -> (Sender, Receiver, Updater) {
    let (send, recv) = tdjson::Client::new().split();
    let (tx, rx) = mpsc::channel(256);
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let client = Sender {
        tdclient: send,
        pending: pending.clone(),
        next_id: Arc::new(AtomicUsize::new(0)),
    };
    let updater = Updater {
        recv: Some(recv),
        tx,
        pending,
    };
    (client, rx, updater)
}

pub type Receiver = mpsc::Receiver<Update>;

pub struct Sender {
    tdclient: tdjson::SendClient,
    pending: Arc<Mutex<HashMap<usize, oneshot::Sender<String>>>>,
    next_id: Arc<AtomicUsize>,
}
pub struct Updater {
    recv: Option<tdjson::ReceiveClient>,
    tx: mpsc::Sender<Update>,
    pending: Arc<Mutex<HashMap<usize, oneshot::Sender<String>>>>,
}
impl Updater {
    pub async fn drive(mut self) {
        let mut recv = self.recv.take();
        loop {
            let (raw, recv2) = blocking::unblock(move || {
                let mut recv2 = recv.unwrap();
                let raw = recv2.receive(Duration::from_secs(1)).map(|r|r.to_owned());
                (raw, recv2)
            }).await;
            recv = Some(recv2);
            let raw = match raw {
                Some(raw) => raw,
                None => continue,
            };
            let mess: Result<Message, _> = serde_json::from_str(&raw);
            match mess {
                Ok(m) => {
                    log::info!("Updater received: {:?}", m);
                    match m {
                        Message::Response(r) => {
                            let mut map =  self.pending.lock().await;
                            let tx = map.remove(&r.id);
                            if let Some(tx) = tx {
                                tx.send(raw.to_owned()).expect("canceled future");
                            } else {
                                log::error!("no request mapped for id {}", r.id);
                            }
                        }
                        Message::Update(u) => {
                            self.tx.send(u).await.expect("canceled future");
                        }
                    }
                }
                Err(e) => {
                    error!("unhandled message: {}", raw);
                    error!("reason: {:?}",e);
                    continue;
                }
            }
        }
    }
}
impl Sender {
    pub async fn send<T: Method>(&self, data: T) -> Result<T::Response, Error> {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let req = Request {
            id,
            payload: data.tag()
        };
        let s = serde_json::to_string(&req).expect("Cannot serialize");
        let (tx, rx) = oneshot::channel();
        {
            let mut map = self.pending.lock().await;
            map.insert(id, tx);
        }
        self.tdclient.send(&s);
        let raw = rx.await.expect("canceled future");
        match serde_json::from_str::<Response>(&raw) {
            Ok(r) => {
                let res = match serde_json::from_value::<T::Response>(r.payload) {
                    Ok(ok) => ok,
                    Err(_) => return Err(Error{code:-1, message: format!("cannot parse response: {}", raw)}),
                };
                Ok(res)
            },
            Err(_) => {
                match serde_json::from_str(&raw) {
                    Ok(ok) => Err(ok),
                    Err(_) => Err(Error{code:-1, message: format!("cannot parse response: {}", raw)}),
                }
            }
        }
    }
}
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
                allow_flash_call: false,
                is_current_phone_number: false,
            };
            sender.send(s).await?;
            wait_for_authorization_state!(receiver, AuthorizationStateWaitCode);
            let s = CheckAuthenticationCode {
                code: (getcode)(),
                first_name: "".to_owned(),
                last_name: "".to_owned(),
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
