#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate futures as original_futures;
extern crate telegram_bot;
extern crate tokio_core;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate hyper;
extern crate futures_await as futures;

use std::env;
use tokio_core::reactor::Core;
use telegram_bot::{Api, UpdateKind, MessageKind, CanReplySendMessage, Update, Error};
use futures::{Stream};
use hyper::client::{Client, HttpConnector};
use hyper::Uri;
use std::sync::Arc;
use futures::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ResponseType {
    ProgramCompiled { program: String, rustc: String },
    ProgramCompileError { rustc: String }
}

#[derive(Serialize)]
pub struct PlaygroundRequest {
    code: String,
    version: String,
    optimize: String,
    test: bool,
    separate_output: bool,
    color: bool,
    backtrace: String
}

#[async]
fn send_request(client: Arc<Client<HttpConnector>>) -> Result<String, ()> {
    let response = await!(client.get("https://hyper.rs".parse::<Uri>().unwrap()));
    let response = response.unwrap();
    let mut body = await!(response.body().concat2());
    let mut body = body.unwrap();
    let result : ResponseType = serde_json::from_slice(&body)
        .unwrap_or(ResponseType::ProgramCompileError {
            rustc: String::from("Ответ на запрос не удалось десериализовать")
        });
    let mut result = match result {
        ResponseType::ProgramCompiled { program, .. } => {
            format!("Программа скомпилированна успешно: {}", program)
        },
        ResponseType::ProgramCompileError { rustc, ..} => {
            format!("Ошибка компиляции программы: {}", rustc)
        }
    };
    if result.len() > 500 {
        result.truncate(500);
    }
    Ok(result)
}

#[async]
fn update_stream(api: Arc<Api>, client: Arc<Client<HttpConnector>>) -> Result<(), Error> {
    let stream = api.stream();
    #[async]
    for update in stream {
        if let UpdateKind::Message(message) = update.kind {
            if let MessageKind::Text { ref data, .. } = message.kind {
                println!("<{}>: {}", &message.from.first_name, data);
                if data.starts_with("/rust ") {
                    let response = await!(send_request(client.clone()));
                    api.spawn(message.text_reply(
                        format!("Hi, {}! You just wrote '{}'", &message.from.first_name, data)
                    ));
                }
            }
        }
    }
    Ok(())
}


fn main() {
    let mut core = Core::new().unwrap();
    let token = env::var("TELEGRAM_BOT_TOKEN").unwrap();
    let api = Arc::new(Api::configure(token).build(core.handle()).unwrap());
    let client = Arc::new(Client::new(&core.handle()));
    let future = update_stream(api.clone(), client.clone());
    core.run(future).unwrap();
}