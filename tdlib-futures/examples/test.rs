extern crate dotenv;
extern crate futures;
extern crate serde_json;
extern crate tdlib_futures;
extern crate tokio_core;

use dotenv::dotenv;
use futures::prelude::*;
use tdlib_futures::client::AuthParameters;
use tdlib_futures::client::Client;
use tdlib_futures::types::*;

fn main() {
    dotenv().ok();
    tdlib_futures::set_log_verbosity_level(1);

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let mut client = Client::new();
    let getcode = || {
        println!("getcode:");
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).expect("no input");
        line.trim().to_owned()
    };
    let tdlib = TdlibParameters {
        use_test_dc: false,
        database_directory: "db".to_owned(),
        files_directory: "Files".to_owned(),
        use_file_database: true,
        use_chat_info_database: true,
        use_message_database: true,
        use_secret_chats: false,
        api_id: std::env::var("TDLIB_API_ID").unwrap().parse().unwrap(),
        api_hash: std::env::var("TDLIB_API_HASH").unwrap().to_owned(),
        system_language_code: "en".to_owned(),
        device_model: "Desktop".to_owned(),
        system_version: "Unknown".to_owned(),
        application_version: "1.0".to_owned(),
        enable_storage_optimizer: true,
        ignore_file_names: false,
    };
    let params = AuthParameters {
        tdlib,
        encryption_key: std::env::var("TDLIB_ENCRYPTION_KEY").unwrap().to_owned(),
        phone: std::env::var("TDLIB_PHONE").unwrap().to_owned(),
        getcode,
    };
    let auth = client.authorize(handle.clone(), params).unwrap();
    let chat_id = std::env::var("TG_USER").unwrap().parse().unwrap();
    let updates = auth.and_then(|updater| {
        let content = InputMessageText {
            text: "test".to_owned(),
            disable_web_page_preview: false,
            clear_draft: false,
        };
        let msg = SendMessage {
            chat_id: chat_id,
            reply_to_message_id: 0,
            disable_notification: false,
            from_background: false,
            reply_markup: None,
            input_message_content: InputMessageContent::InputMessageText(content),
        };
        let msg = client.send(msg).and_then(|r| {
            println!("response: {:?}",r);
            Ok(())
        }).map_err(|_|());
        handle.spawn(msg);
        updater.for_each(|u| {
            println!("new update: {:?}",u);
            Ok(())
        })
    });

    core.run(updates).unwrap();
}
