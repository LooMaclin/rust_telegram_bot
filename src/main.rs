extern crate telegram_bot;
extern crate hyper;
extern crate hyper_rustls;
extern crate serde_json;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use telegram_bot::{Api, MessageType, ListeningMethod, ListeningAction};
use std::io::Read;
use hyper::client::Client;
use hyper::net::HttpsConnector;


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

fn main() {
    // Create bot, test simple API call and print bot information
    let api = Api::from_env("TOKEN").unwrap();
    println!("getMe: {:?}", api.get_me());
    let mut listener = api.listener(ListeningMethod::LongPoll(None));

    let res = listener.listen(|u| {

        if let Some(m) = u.message {
            let name = m.from.first_name;
            match m.msg {
                MessageType::Text(t) => {
                    println!("<{}> {}", name, t);

                    if t.starts_with("/rust ") {
                        let program = t.split("/rust ").collect();
                        let mut result = String::new();
                        let tls = hyper_rustls::TlsClient::new();
                        let connector = HttpsConnector::new(tls);
                        let client = Client::with_connector(connector);
                        let playground_request = serde_json::to_string(&PlaygroundRequest {
                            code: program,
                            version: String::from("stable"),
                            optimize: String::from("0"),
                            test: false,
                            separate_output: true,
                            color: false,
                            backtrace: String::from("0"),
                        }).unwrap();
                        let mut response = client.post("https://play.rust-lang.org/evaluate.json")
                            .body(&playground_request)
                            .send()
                            .unwrap();
                        response.read_to_string(&mut result);
                        println!("Result : {:?}", result);
                        let result : ResponseType = serde_json::from_str(&result)
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
                        try!(api.send_message(
                            m.chat.id(),
                            result,
                            None, None, Some(m.message_id), None));
                    }
                },
                _ => {}
            }
        }
        Ok(ListeningAction::Continue)
    });

    if let Err(e) = res {
        println!("An error occured: {}", e);
    }
}