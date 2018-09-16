extern crate tdlib_futures;
extern crate futures;
extern crate tokio_core;
#[macro_use]
extern crate serde_json;

use tdlib_futures::client::Client;
use tdlib_futures::types::*;
use futures::prelude::*;

fn main() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let mut client = Client::new();
    let auth = client.authorize(handle.clone()).unwrap();

    let chat_id = 57333322;
    let updates = auth.and_then(|updater| {
        let content = InputMessageText {
            text: FormattedText {text: "test".to_owned()},
        };
        let msg = SendMessage {
            chat_id: chat_id,
            reply_to_message_id: 0,
            disable_notification: false,
            from_background: false,
            input_message_content: InputMessageContent::InputMessageText(content),
        };
        client.send_spawn(msg, &handle);
        updater.for_each(|u| {
            match u {
                _ => {}
            }
            Ok(())
        })
    });

    core.run(updates).unwrap();
}
