---
title: "Long-running backend async tasks in tauri v2"
date: 2024-05-12
slug: "long-running-backend-async-tasks-in-tauri-v2"
summary: "How to run backend async tasks in tauri v2 and push data to the frontend"
category: "tech"
---

## Introduction

Welcome to our tutorial on running backend async tasks in [Tauri v2](https://beta.tauri.app) and seamlessly pushing data to the frontend! In this guide, we'll delve into the process of orchestrating asynchronous backend operations while synchronously updating the frontend. Our example project revolves around crafting a Twitch Chat application, where we'll establish a connection to Twitch's IRC, fetch messages, and dynamically present them within our application interface.

The core of our approach lies in leveraging Tauri's robust event system to seamlessly bridge the gap between backend processes and frontend presentation. By the end of this tutorial, you'll have a clear understanding of how to synchronize backend async tasks with frontend data updates, empowering you to build responsive and dynamic applications with Tauri.

### Notes

For this tutorial, we'll be using Tauri version `2.0.0-beta`. Because this is a beta, some of the code may change in the future.

## Initializing Project

To kickstart our journey, ensure you have Tauri v2 and its associated tooling installed. If not, you can swiftly set it up by installing create-tauri-app using Cargo:

```sh
cargo install create-tauri-app
```

Then, initialize your Tauri application using:

```sh
cargo create-tauri-app --beta
```

While we'll be utilizing React for the frontend in this tutorial, feel free to adapt the frontend framework of your choice. The crux of our focus, however, will lie in configuring and executing backend tasks efficiently.

**[Explore alternative installation methods via Tauri's official documentation](https://beta.tauri.app/start/create-project/)**

For a visual walkthrough of the setup process, watch the following clip:

<script src="https://asciinema.org/a/Y14AWJ8No92Hm4upWYawQmeOJ.js" id="asciicast-656442" async="true"></script>

## Sending Data from Backend

Now that our project is primed, let's dive into creating long-running async tasks capable of dispatching data to the frontend. Our objective? Establish a connection to Twitch's chat service, continuously fetch incoming messages, and seamlessly relay them to our application's frontend.

We'll achieve this by spawning an async task that orchestrates the connection to Twitch's IRC, diligently processing incoming messages. Additionally, we'll equip this task with the capability to dispatch these messages to the frontend using event handlers.

To streamline your setup, we'll specify the necessary dependencies, demonstrate how to initialize the Twitch IRC connection within the chat module, and configure the Tauri builder to seamlessly integrate our newly minted chat module.

Add `twitch-irc` as a dependency in `src-tauri/Cargo.toml`. Here's all the dependencies we have at this point:

```toml
[build-dependencies]
tauri-build = { version = "2.0.0-beta", features = [] }

[dependencies]
tauri = { version = "2.0.0-beta", features = [] }
tauri-plugin-shell = "2.0.0-beta"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
twitch-irc = "5.0.1"
```

Next, we'll add basic functionality for connecting to a target channel and reading the chat. The code is mostly just copied from the example on the [twitch-irc README](https://github.com/robotty/twitch-irc-rs/).

The setup itself is meant to be called within an async task, and it will also spawn a second async task which bubble data up to our handler

```rust
use tauri::{async_runtime, EventTarget};
use tauri::{AppHandle, Manager};
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::ServerMessage;
use twitch_irc::ClientConfig;
use twitch_irc::SecureTCPTransport;
use twitch_irc::TwitchIRCClient;

#[derive(Clone, serde::Serialize)]
struct TwitchMessagePayload {
    id: String,
    text: String,
    sender: String,
    channel: String,
}

pub async fn monitor_chat(
    handle: &AppHandle,
    target_channel: String,
) -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    let config = ClientConfig::default();
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

    let twitch_chat_app_handler = handle.clone();
    let join_handle = async_runtime::spawn(async move {
        let handler = twitch_chat_app_handler;
        while let Some(message) = incoming_messages.recv().await {
            match message {
                ServerMessage::Privmsg(msg) => {
                    let payload = TwitchMessagePayload {
                        id: msg.message_id,
                        text: msg.message_text,
                        sender: msg.sender.login,
                        channel: msg.channel_login,
                    };
                    let _result =
                        handler.emit_to(EventTarget::app(), "twitch_message_received", payload);
                }
                _ => {}
            }
        }
    });

    client.join(target_channel).unwrap();
    join_handle.await.unwrap();
    Ok(())
}
```

Lastly, we're going to have tauri run this function. We could have the function run right when tauri starts up, but we'll more than likely miss the first set of messages before the frontend instantiates.

An approach I took was to have the frontend send a message to the backend to start the chat monitor. This way, we can ensure that the frontend is ready to receive messages.

More explicitly, when the frontend finishes loading it sends an event called `track_stream` to the backend with the streamer's name. The backend then starts the chat monitor for that streamer and sends messages to the frontend via the `twitch_message_received` event that we defined earlier.

```rust
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use tauri::Manager;
mod chat;

#[derive(Serialize, Deserialize, Debug)]
struct StartStreamPayload {
    streamer: String
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
                window.close_devtools();
            }
            let handler_clone = app.handle().clone();
            app.listen("track_stream", move |event| {
                let handler_clone = handler_clone.to_owned();
                let payload = event.payload();
                let parsed_payload: StartStreamPayload = serde_json::from_str(payload)
                .expect("Could not parse track stream payload");
                tauri::async_runtime::spawn(async move {
                    let _result = chat::setup_chat_monitor(&handler_clone, parsed_payload.streamer).await;
                });
            });
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

```

## Displaying Data with Frontend

The frontend is fairly standard React code. When the app starts, it sends a message to the backend to start the chat monitor via the `track_stream` event.

That starts the chat monitor, and the backend sends messages to the frontend via the `twitch_message_received` event. The frontend listens for this event and updates the chat window with the new message.

```typescript
type Message = {
  id: string;
  text: string;
  sender: Sender;
  channel: string;
};

export const App = () => {
  const [targetStream, setTargetStream] = useState<string>("");
  const [messages, setMessages] = useState<Message[]>([]);
  useEffect(() => {
    if (!targetStream) {
      return;
    }
    // Initialize the stream and start listening for messages
    emit("track_stream", { streamer: targetStream }).catch((err) =>
      console.error(`could not track stream ${err}`),
    );
    const unlisten = listen<Message>("twitch_message_received", (event) => {
      const message: Message = {
        id: event.payload.id,
        text: event.payload.text,
        sender: event.payload.sender,
        channel: event.payload.channel,
      };
      setMessages((prevMsgs) => {
        return [...prevMsgs, message];
      });
    }).catch((err) => console.error(`could not receive messages ${err}`));

    return () => {
      // @ts-ignore
      unlisten.then((f) => f());
    };
  }, [targetStream]);

  return (
    <div>
      <input
        type="text"
        value={targetStream}
        onChange={(e) => setTargetStream(e.target.value)}
      />
      <div>
        {messages.map((msg) => (
          <div key={msg.id}>
            <p>{msg.sender}</p>
            <p>{msg.text}</p>
          </div>
        ))}
      </div>
    </div>
  );
  )
}
```

**Caveat: The code above is simplified, and specifically doesn't cover _stopping_ the chat monitor when the user changes the streamer.**

## Conclusion

And that's it! We've successfully set up a Twitch chat monitor using Rust and Tauri. We've covered how to set up the backend to listen for messages and how to send messages to the frontend. We've also covered how to set up the frontend to listen for messages and update the UI accordingly.
