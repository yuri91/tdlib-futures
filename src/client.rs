use ::futures::Stream;
use ::futures::Future;
use ::futures::sync::mpsc;
use ::futures::sync::oneshot;
use ::serde::Serialize;
use ::std::sync::Arc;
use ::std::sync::Mutex;
use ::std::sync::atomic::AtomicUsize;
use ::std::collections::HashMap;
use ::std::time::Duration;
use ::std;
use ::serde_json;
use ::tokio_core;
use ::futures;

use super::tdjson;
use super::types::*;

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
}

#[derive(Clone)]
pub struct Client {
    tdclient: Arc<tdjson::Client>,
    rx: Arc<Mutex<Option<mpsc::UnboundedReceiver<Update>>>>,
    pending: Arc<Mutex<HashMap<usize, oneshot::Sender<String>>>>,
    next_id: Arc<AtomicUsize>,
}
impl Client {
    pub fn new() -> Client {
        let tdclient = Arc::new(tdjson::Client::new());
        let (tx, rx) = mpsc::unbounded();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let client = Client {
            tdclient: tdclient.clone(),
            rx: Arc::new(Mutex::new(Some(rx))),
            pending: pending.clone(),
            next_id: Arc::new(AtomicUsize::new(0)),
        };
        std::thread::spawn(move || {
            let mut running = true;
            while running {
                let raw = tdclient.receive(Duration::from_secs(1));
                let raw = match raw {
                    Some(raw) => raw,
                    None => continue,
                };
                let mess: Result<Message, _> = serde_json::from_str(raw);
                match mess {
                    Ok(m) => {
                        match m {
                            Message::Response(r) => {
                                let mut map =  pending.lock().unwrap();
                                let tx = map.remove(&r.id);
                                if let Some(tx) = tx {
                                    let _ = tx.send(raw.to_owned());
                                }
                            }
                            Message::Update(u) => {
                                let sent = tx.unbounded_send(u);
                                running = sent.is_ok();
                                // TODO send error to channel
                            }
                        }
                    }
                    Err(_) => {
                        println!("unhandled message: {}", raw);
                        continue;
                    }
                }
            }
        });
        client
    }
    fn do_send<T: Method>(&self, data: T) -> oneshot::Receiver<String> {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let req = Request {
            id,
            payload: data.tag()
        };
        let s = serde_json::to_string(&req).expect("Cannot serialize");
        println!("sending: {}",s);
        let (tx, rx) = oneshot::channel();
        let mut map = self.pending.lock().unwrap();
        map.insert(id, tx);
        self.tdclient.send(&s);
        rx
    }
    pub fn authorize(&mut self, handle: tokio_core::reactor::Handle) -> Option<impl Future<Item = Updater, Error = ()>> {
        let mut rx = self.rx.lock().unwrap();
        rx.take().map(|rx| {
            Authorization {
                rx: Some(rx),
                client: self.clone(),
                handle,
            }
        })
    }
    pub fn send<T: Method>(&self, data: T) -> impl Future<Item=T::Response, Error=futures::Canceled> {
        AsyncResponse {
            data,
            client: self.clone(),
            inner: None,
        }
    }
    pub fn send_spawn<T: Method+'static>(&self, data: T, handle: &tokio_core::reactor::Handle) {
        let f = self.send(data);
        handle.spawn(f.map(|_|()).map_err(|_|()));
    }
}
struct AsyncResponse<T: Method> {
    data: T,
    client: Client,
    inner: Option<oneshot::Receiver<String>>
}
impl<T: Method> Future for AsyncResponse<T> {
    type Item = T::Response;
    type Error = futures::Canceled;
    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        if self.inner.is_none() {
            self.inner = Some(self.client.do_send(self.data.clone()));
        }
        match self.inner.as_mut().unwrap().poll() {
            Ok(futures::Async::NotReady) => Ok(futures::Async::NotReady),
            Ok(futures::Async::Ready(r)) => {
                println!("received: {}",r);
                let resp = serde_json::from_str(&r).unwrap();
                Ok(futures::Async::Ready(resp))
            },
            Err(e) => Err(e)
        }
    }
}
pub struct Authorization {
    rx: Option<mpsc::UnboundedReceiver<Update>>,
    client: Client,
    handle: tokio_core::reactor::Handle,
}
impl Future for Authorization {
    type Item = Updater;
    type Error = ();
    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        if self.rx.is_none() {
            panic!("Already authorized");
        }
        {
            let rx = self.rx.as_mut().unwrap();
            'l: loop {
                match rx.poll() {
                    Err(_) => {
                        return Err(());
                    },
                    Ok(a) => {
                        match a {
                            futures::Async::Ready(u) => {
                                if u.is_none() {
                                    return Err(());
                                }
                                match u.unwrap() {
                                    Update::UpdateAuthorizationState {
                                        authorization_state,
                                    } => {
                                        match authorization_state {
                                            UpdateAuthorizationState::AuthorizationStateWaitTdlibParameters => {
                                                let s = SetTdlibParameters {
                                                    parameters: TdlibParameters {
                                                        use_test_dc: false,
                                                        api_id: 171315,
                                                        api_hash: "ab1d086610068dea947a5ffd7028cbce".to_owned(),
                                                        device_model: "Desktop".to_owned(),
                                                        system_version: "Unknown".to_owned(),
                                                        application_version: "0.0".to_owned(),
                                                        system_language_code: "en".to_owned(),
                                                        files_directory: "Files".to_owned(),
                                                        use_chat_info_database: true,
                                                        use_message_database: true,
                                                    },
                                                };
                                                self.client.send_spawn(s, &self.handle);
                                            }
                                            UpdateAuthorizationState::AuthorizationStateWaitEncryptionKey => {
                                                let s = CheckDatabaseEncryptionKey {
                                                    encryption_key: "".to_owned(),
                                                };
                                                self.client.send_spawn(s, &self.handle);
                                            }
                                            UpdateAuthorizationState::AuthorizationStateWaitPhoneNumber => {
                                                let s = SetAuthenticationPhoneNumber {
                                                    phone_number: "310646493160".to_owned(),
                                                    allow_flash_call: false,
                                                    is_current_phone_number: false,
                                                };
                                                self.client.send_spawn(s, &self.handle);
                                            }
                                            UpdateAuthorizationState::AuthorizationStateWaitCode => {
                                                let mut line = String::new();
                                                std::io::stdin().read_line(&mut line).expect("no input");
                                                let s = CheckAuthenticationCode {
                                                    code: line.trim().to_owned(),
                                                };
                                                self.client.send_spawn(s, &self.handle);
                                            }
                                            UpdateAuthorizationState::AuthorizationStateWaitPassword => {
                                                //TODO
                                            }
                                            UpdateAuthorizationState::AuthorizationStateReady => {
                                                break 'l;
                                            }
                                        }
                                    }
                                    _ => {
                                    },
                                }
                            },
                            futures::Async::NotReady => {
                                return Ok(futures::Async::NotReady);
                            }
                        }
                    }
                }
            }
        }
        Ok(futures::Async::Ready(Updater {
            rx: self.rx.take().unwrap(),
        }))
    }
}
pub struct Updater {
    rx: mpsc::UnboundedReceiver<Update>,
}
impl Stream for Updater {
    type Item = Update;
    type Error = ();
    fn poll(&mut self) -> futures::Poll<Option<Self::Item>, Self::Error> {
        self.rx.poll()
    }
}
