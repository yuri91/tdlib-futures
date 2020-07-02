use dotenv::dotenv;
use futures::prelude::*;
use futures::task::SpawnExt;
use tdlib_futures::client::init;
use tdlib_futures::utils::{authorize, AuthParameters};
use tdlib_futures::types::*;
use tdlib_futures::methods::*;

fn main() {
    dotenv().ok();
    env_logger::init();
    tdlib_futures::set_log_verbosity_level(1);

    let mut pool = futures::executor::LocalPool::new();
    let (mut sender, mut receiver, updater) = init();
    let spawner = pool.spawner();
    spawner.spawn(updater.drive()).expect("cannot spawn updater");
    let tdlib = TdlibParameters {
        use_test_dc: false,
        database_directory: "data/db".to_owned(),
        files_directory: "data/Files".to_owned(),
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
    let getcode = || {
        println!("getcode:");
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).expect("no input");
        line.trim().to_owned()
    };
    let params = AuthParameters::for_bot(tdlib,
        std::env::var("TDLIB_ENCRYPTION_KEY").unwrap().to_owned(),
        std::env::var("TDLIB_BOT_TOKEN").unwrap().to_owned());
    //let params = AuthParameters::for_user(tdlib,
    //    std::env::var("TDLIB_ENCRYPTION_KEY").unwrap().to_owned(),
    //    std::env::var("TDLIB_PHONE").unwrap().to_owned(),
    //    getcode);
    let my_id: i32 = std::env::var("TG_BOT_ID").unwrap().parse().unwrap();
    pool.run_until(async move {
        authorize(params, &mut sender, &mut receiver).await.expect("failed to authorize");
        loop {
            let update = dbg!(receiver.next().await);
            if let Some(Update::UpdateNewMessage(msg)) = update {
                if msg.message.sender_user_id == my_id {
                    continue;
                }
                if let MessageContent::MessageText(text) = msg.message.content {
                    let m = InputMessageText {
                        text: FormattedText {
                            text: format!("echo '{}'", text.text.text),
                            entities: Vec::new(),
                        },
                        clear_draft: false,
                        disable_web_page_preview: true,
                    };
                    let resp = SendMessage {
                        chat_id: msg.message.chat_id,
                        reply_to_message_id: msg.message.id,
                        disable_notification: false,
                        from_background: false,
                        reply_markup: None,
                        input_message_content: InputMessageContent::InputMessageText(m),
                    };
                    dbg!(sender.send(resp).await).ok();
                }
            }
        }
    });
}
