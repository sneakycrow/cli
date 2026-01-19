---
title: Playing with Gleam
date: 2024-06-13
slug: playing-with-gleam
summary: ' A high level exploration of the gleam language, a new programming language'
category: tech
edits:
- id: 4d68017c514e629f97bac8e53811b6c31e9086b6
  message: add reading section
  author: sneakycrow
  timestamp: '1718581137'
- id: 6b9487f981abd0d5ab7ae5acf1538f815f78efa3
  message: add categories to posts
  author: Zachary Corvidae
  timestamp: '1720820568'
- id: 2e8c6cb16472096a2ff3ce3419c72e3fc3b562b3
  message: Sneakycrow/better-syntax-highlighting (#265)
  author: Zachary Corvidae
  timestamp: '1719365907'
---

## what is gleam?

> **Gleam is a friendly language for building type-safe systems that scale! -** [https://gleam.run/](https://gleam.run/)

## why?

Right now, two of my favorite languages to work with are Rust and Typescript. Generally, I prefer Rust for it’s type features. I especially like it’s ownership model, and find the Rust compiler to be a particularly enjoyable feature.

Typescript is more of a product of preferring to develop UI’s in JS frameworks still, but wanting to keep that safety. I use frameworks like Svelte, React, and Vue and prefer TS type system when developing frontend applications.

I like Rust, but I find it somewhat painful to work with still when quickly prototyping things. Right now, I reach for languages like Go for quick prototyping, but when I saw Gleam’s syntax, particularly it’s pipe operator and case matching, which I dearly miss from Rust when using enum’s in Go, I knew I wanted to try it out.

## what’s good?

### Interoperability with JS

One really interesting feature is [being able to compile Gleam to JS](https://gleam.run/news/v0.16-gleam-compiles-to-javascript/). In the past, with languages like Rust, you commonly see exporting to WASM. So I thought it was particularly interesting to get the promise of type safety but while compiling to vanilla JS.

This allows the backend and frontend of a web stack share more code, for example. While still allowing the frontend to use JS and the backend to get the advantage of the BEAM runtime.

I can’t foresee myself replacing frameworks like React or Svelte yet, but I can easily see myself writing a backend and exporting some shared functionality utilities for the frontend to take advantage of. And being able to share those common patterns has an interesting effect of business logic staying surprisingly in-sync even within silo’s.

### Pipe Operators

The pipe operators are really pleasant to use. Once I grasped how they worked, I found myself trying to take advantage of them wherever I could. I think even just syntactically they’re really pleasant looking.

As I understand it, the return value of the first function is passed as the first argument into the piped function. Here's a function from my [gwitch](https://hex.pm/packages/gwitch) project that utilizes it a few times. Particularly on the last line, where the

```gleam
/// A function for connecting to a twitch channel. Returns a subject for sending and receiving messages
pub fn connect(config: Config, channel: String) -> Subject(Message) {
  // Establish a connection to the Twitch server and create the builder
  let assert Ok(req) = request.to(twitch_server)
  // Create a wrapper around the receive handler that translates the string to a message
  let on_receive = fn(msg) { to_twitch_message(msg) |> config.on_receive }
  let assert Ok(subj) =
    websockets.create_websockets_builder(req, Some(on_receive))
  // Log in to the Twitch server and join the channel
  login(subj, config) |> join_channel(channel)
}
```

But also when you have a waterfall pattern, which I run into a lot, they just make a ton of sense.

It reminds me of the Rust builder pattern, but with that I felt more of a need to keep some static type that was mutating as it was passed down through the chain. Where as these pipe operators feel more flexible and loose, but not so loose I feel I’m going to shoot myself in the foot.

## my thoughts today

It’s fun to see a language developing, and I’ve started working on a few public libraries within Gleam’s ecosystem. The language has enjoyable patterns and syntax that allows really quick and fluid prototyping. I don’t find myself often shooting myself in the foot, nor do I find myself being blocked very frequently.

But it is a very fresh language. The documentation feels generally weak to me. I find myself commonly reading source code.

The ecosystem itself is fresh, but only in the way of Gleam specific libs. I believe you can take advantage of Elixer libs, which has a large ecosystem. As of this writing I haven’t totally grasped how to take advantage of those packages.

Overall, I am enjoying the language and plan on continuing to use it. For production projects, I’ll probably still reach for Rust or TS. But for fun quick projects, Gleam will be part of my toolbox.
