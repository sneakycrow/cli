---
title: "Pushing notifications to the client from Postgres"
date: 2024-10-21
category: "tech"
summary: "Pushing postgres `LISTEN` events to a web client via websockets with Rust"
series_key: "video-streaming-devlog"
series_pos: 2
---
In our [last post](https://sneakycrow.dev/blog/2024-10-15-creating-a-queue-for-video-processing) we created a queue system to offload processing of our raw videos from the API. But, in doing so, we
made it so the API returns the response well before the video is done processing.

Now, we need a way to communicate back to the client when a video is done processing. We'll achieve this by taking advantage
of a Postgres' [LISTEN](https://www.postgresql.org/docs/current/sql-listen.html) directive.

We're going to create a trigger that watches for updates to the `processing_status` column on the `videos` table
we created. Then, we'll create a websockets connection from client to server, and when we receive a notification
from Postgres for the requested video, we pass that notification to the websockets client.

### Adding a notify trigger

First, we'll need to create a migration that adds a trigger to the `videos` tables' `processing_status` column. Whenever that column
gets updated, we want Postgres to notify us.

```sql 0003_upload_status_notification.sql
-- First, let's create the function to notify status changes
CREATE OR REPLACE FUNCTION notify_upload_status()
RETURNS TRIGGER AS $$
BEGIN
    -- Use the processing_status column directly
    PERFORM pg_notify('upload_status', NEW.id || ',' || NEW.processing_status);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Now, let's create the trigger
CREATE TRIGGER upload_status_trigger
AFTER UPDATE OF processing_status ON videos
FOR EACH ROW
EXECUTE FUNCTION notify_upload_status();
```

If you're using sqlx, you can run this with `sqlx migrate run`

### Setting up websockets

Next, we'll want to establish a means of communication between our API and the web client.
For this tutorial, we're using [axum](https://github.com/tokio-rs/axum) and [askama](https://github.com/djc/askama),
but this should work with any frontend and any framework that supports websockets.
The reason we're using websockets is mainly so we can push an event from the server to the client, instead of
making the client request the event.

We'll create a new route and handler for the websockets connection called `/events`

```rust main.rs
// ... omitted for brevity
.route("/events", get(event_handler))
// ... omitted for brevity
```

And then we create a handler that upgrades the websocket connection and passes it to another handler function that can utilize
the socket.

```rust main.rs
pub async fn event_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}
```

Alright, admittedly this part is a little confusing. But bear with me 🐻. What we need to do is create two "connections", and then
link those connections together. One connection will be communicating with Postgres and listening for notifications.
When it receives a notification, it'll parse it and pass it to the websockets connection, which will then push it to the client.
We _could_ also skip having the `ev_` and just pass the events directly to websockets, but
doing it this way allows us to extend it later for different kind of events. The ultimate goal is for this
to be a event stream of multiple kinds of events.

We'll start by defining the senders/receivers, one for the websockets connection (using the socket) and another one for Postgres
using a broadcast channel.

_Note: Broadcast sends all messages to **every** receiver. You'll want to probably modify some of this behavior for production
settings_

```rust events.rs
/// A websocket handler that passes upload_status events to the client
/// NOTE: This currently sends _all_ notifications to the subscribed client
/// TODO: Add authorization and filter only events the client cares about
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    // Split our websocket sender/receiver for communication with the client
    let (mut sender, mut receiver) = socket.split();

    // Create a new sender/receiver for sending/receiving events from Postgres
    let (ev_sender, mut ev_receiver): (broadcast::Sender<Event>, broadcast::Receiver<Event>) =
        broadcast::channel(100); // You can probably reduce this, I just pulled it from an example

    // ... omitted for brevity
}
```

Okay, now what we'll do is set up our `ev_` recevier/sender to communicate with Postgres. Using tokio, we'll spawn
this in a task so it can run concurrently with our websockets sender/receiver.

When we receive a notification from Postgres we'll also want to parse it into our `Event` struct
so we can serialize it for the client.

We're going to take advantage of [sqlx PgListener](https://docs.rs/sqlx/latest/i686-unknown-linux-gnu/sqlx/postgres/struct.PgListener.html).
This gives us some niceties like auto-reconnecting if the connection dies.

If you're using sql directly, you can use the [Listen feature](https://www.postgresql.org/docs/current/sql-listen.html).

```rust events.rs
// ... omitted for brevity
// Spawn a task for listening for notifications from Postgres on the upload_status trigger
tokio::spawn(async move {
    // Start the listener
    let mut listener = PgListener::connect_with(&state.db).await.unwrap();
    listener.listen("upload_status").await.unwrap();
    tracing::debug!("Listening for upload status changes");

    // When we receive a notification, parse it into an Event and pass it to the sender
    while let Ok(notification) = listener.recv().await {
        let payload = notification.payload();
        let parts: Vec<&str> = payload.split(',').collect();
        tracing::debug!(
            "Notification received! Payload: {:?}, Parts: {:?}",
            payload,
            parts
        );
        if parts.len() == 2 {
            let event = Event::VideoProcessingStatus {
                video_id: parts[0].to_string(),
                status: parts[1].to_string(),
            };
            let _ = ev_sender.send(event);
        }
    }
});
// ... omitted for brevity
```

If you run this as is you should see the debug statement. Now we just need to setup
the `ev_receiver` to pass the events to the websockets channel.

We'll spawn another task that will listen to received events and send them to our
websockets clients. We'll also add the receiver from the websockets client in case
we later need some client communications.

```rust events.rs
// ... omitted for brevity
// Send events to the client
let mut send_task = tokio::spawn(async move {
    while let Ok(event) = ev_receiver.recv().await {
        tracing::debug!("Received parsed event, sending to client");
        let server_message = ServerMessage { event };
        if let Ok(json) = serde_json::to_string(&server_message) {
            if sender
                .send(axum::extract::ws::Message::Text(json))
                .await
                .is_err()
            {
                break;
            }
        }
    }
});
// Parse messages from the client
let mut recv_task = tokio::spawn(async move {
    while let Some(Ok(_message)) = receiver.next().await {
        // Process incoming message from client
    }
});
// ... omitted for brevity
```

Lastly, we'll use the `tokio::select!` macro to exit if any of our connections complete.

```rust events.rs
// If any of the tasks exit, abort the other one
tokio::select! {
    _ = (&mut send_task) => recv_task.abort(),
    _ = (&mut recv_task) => send_task.abort(),
};
```

And that's all there is to it! We now have a events handler that can stream events from
various sources back to our websockets client. Here's the complete function

```rust events.rs

/// The serialized message we send back to the client
#[derive(Serialize)]
struct ServerMessage {
    event: Event,
}

/// A websocket handler that passes upload_status events to the client
/// NOTE: This currently sends _all_ notifications to the subscribed client
/// TODO: Add authorization and filter only events the client cares about
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    // Split our websocket sender/receiver for communication with the client
    let (mut sender, mut receiver) = socket.split();

    // Create a new sender/receiver for sending/receiving events from Postgres
    let (ev_sender, mut ev_receiver): (broadcast::Sender<Event>, broadcast::Receiver<Event>) =
        broadcast::channel(100);

    // Spawn a task for listening for notifications from Postgres on the upload_status trigger
    tokio::spawn(async move {
        // Start the listener
        let mut listener = PgListener::connect_with(&state.db).await.unwrap();
        listener.listen("upload_status").await.unwrap();
        tracing::debug!("Listening for upload status changes");

        // When we receive a notification, parse it into an Event and pass it to the sender
        while let Ok(notification) = listener.recv().await {
            let payload = notification.payload();
            let parts: Vec<&str> = payload.split(',').collect();
            tracing::debug!(
                "Notification received! Payload: {:?}, Parts: {:?}",
                payload,
                parts
            );
            if parts.len() == 2 {
                let event = Event::VideoProcessingStatus {
                    video_id: parts[0].to_string(),
                    status: parts[1].to_string(),
                };
                let _ = ev_sender.send(event);
            }
        }
    });
    // Send events to the client
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = ev_receiver.recv().await {
            tracing::debug!("Received parsed event, sending to client");
            let server_message = ServerMessage { event };
            if let Ok(json) = serde_json::to_string(&server_message) {
                if sender
                    .send(axum::extract::ws::Message::Text(json))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    // Parse messages from the client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(_message)) = receiver.next().await {
            // Process incoming message from client
        }
    });

    // If any of the tasks exit, abort the other one
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
```

[Here's the full repo](https://github.com/sneakycrow/file-nest) with all the functionality
in this series if you want a complete example.
