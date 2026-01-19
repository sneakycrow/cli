---
title: "beautiful rust"
date: 2023-03-16
category: "tech"
summary: "Just me admiring my favorite language"
---

There isn't much to this post other than me just admiring how beautiful of a language rust is. I love it so much.

```rust
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Error;
use tracing::{debug, instrument};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
/// Event is the base type for different kinds of inputs and outputs for the processor. Events are
/// things like text matching requests, trigger requests, status requests, and whatever else.
/// Each type of processable request is in this enumerator
pub(crate) enum Event {
    Check(TextCount),
}

impl Event {
    pub(crate) fn process(self) -> impl Serialize {
        match self {
            Self::Check(text_count) => text_count.digest().unwrap(),
        }
    }
}

/// Consume is meant to imply an event is to be processed
trait Consume {
    /// SerializableResult is generally the data that will be serialized into the axum response body
    type SerializableResult: Serialize;
    /// collect is for gathering the data of a consumable event, like reading in local json
    fn collect(&mut self) -> &mut Self;
    /// digest is for digesting the data gathered and processing it, like counting the matches of text
    fn digest(self) -> Result<Self::SerializableResult, Error>;
}

/// Trigger is meant to imply an event that is going to take an action, such as sending a POST to a webhook
trait Trigger {
    /// SerializableResult is generally the data that will be serialized into the axum response body
    type SerializableResult: Serialize;
    /// dispatch is the action to take when this event (read action) is executed/dispatched
    fn dispatch(self) -> Result<Self::SerializableResult, Error>;
}

/// A function to give to the axum router for handling http POST requests for events
pub(crate) async fn post_event(Json(payload): Json<Event>) -> Json<Value> {
    debug!("Received payload {:?}", payload);
    let result = payload.process();
    Json(json!(result))
}

#[derive(Serialize, Deserialize, Debug)]
/// TextCount is representative of a grouping of patterns of text to check against for each message
pub(crate) struct TextCount {
    target: String,
    patterns: Vec<String>,
}

impl Consume for TextCount {
    type SerializableResult = TextCountResult;
    #[instrument]
    fn collect(&mut self) -> &mut Self {
        debug!("Collecting data for text count");
        self
    }

    #[instrument]
    fn digest(self) -> Result<Self::SerializableResult, Error> {
        // This event consumes itself and processes its associated patterns
        let mut pattern_results: Vec<PatternResult> = vec![];

        for pattern in self.patterns {
            pattern_results.push(PatternResult::new(pattern, 0))
        }

        let result = TextCountResult::new(self.target, pattern_results);
        Ok(result)
    }
}

#[derive(Serialize, Deserialize, Debug)]
/// PatternResult contains the text that was checked for each message and the amount of times it was found
pub(crate) struct PatternResult {
    text: String,
    count: i32,
}

impl PatternResult {
    /// Utility function for generating self
    fn new(text: String, count: i32) -> Self {
        Self { text, count }
    }
}

#[derive(Serialize, Deserialize, Debug)]
/// TextCountResult is the processed result of a [Check(TextCount)](Event::Check)
pub(crate) struct TextCountResult {
    target: String,
    patterns: Vec<PatternResult>,
}

impl TextCountResult {
    /// Utility function for generating self
    fn new(target: String, patterns: Vec<PatternResult>) -> Self {
        Self { target, patterns }
    }
}
```
