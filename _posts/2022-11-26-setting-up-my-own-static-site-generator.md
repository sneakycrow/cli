---
title: "Custom Static Site Generator - planning & initialization"
date: 2022-11-26
category: "tech"
series_key: "website-generator-tutorial"
series_pos: 0
summary: "The initial project setup and planning phase of setting up a static site generator"
---

Recently, I set up this new website re-design. But beyond just a re-design, it's actually a full "re-software"?.
As you may have guessed from the title, I wrote my
own [static site generator for this website](https://github.com/sneakycrow/website), and I'm going to review my
step-by-step process for setting it up.

In this post I'll walk through a simple planning phase, my initial project setup, and basic page generation.

## Planning

My first step in any project is a dedicated planning phase. Too often in the past did I think that flying by the seat of
my pants was going to work out for me. Even when it did, if I properly plan and research what I want to do I have a much
more organized approach, generally, than the latter.

My requirements with jumping off points for implementations:

1. Small static files as my output
   1. Simple binary built in Rust that I can run that produces the html files
2. Support for local markdown files parsed into blog posts
   1. [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) for parsing markdown into html
   2. [handlebars](https://github.com/sunng87/handlebars-rust) for page templating
3. Support for SASS
   1. [grass](https://github.com/connorskees/grass) for parsing with Rust
4. Support for simple assets (images and fonts)
   1. [walkdir](https://github.com/BurntSushi/walkdir) for walking directories to copy assets
5. Automatic Deployment
   1. [GitHub s3 deploy + cloudfront cache invalidation action](https://github.com/Reggionick/s3-deploy) for deploying
      to S3 and invalidating cache
6. Dynamic UI components using React (as the "templating" language)
   1. [preact](https://preactjs.com/) for a small basic react implementation
      1. using the [htm](https://github.com/developit/htm) library, so we don't need a js build step

## Setting up the environment

I like to use Cargo workspaces for most of my Rust projects. Even if I only intend for a single binary at first, I find
already having the environment configured for workspaces. In this particular instance, it ended up being handy because I
ended up wanting a custom simple dev server for local development (so the project has a cli and a server now, even
though the server isn't used in prod).

First, I create a root folder and an empty Cargo.toml with it.
`mkdir website && cd website && touch Cargo.toml`

Next, we'll create our initial Cargo project, the cli.
`cargo new cli`

Afterwards, we want to initialize our workspace `Cargo,toml`, which should look something like this:

```toml
[workspace]

members = [
    "cli",
]

default-members = ["cli"]
```

I added the `default-members` key with cli in it as well, as this makes it so when you run `cargo run` it knows to use
the cli binary. From here, you should be able to do `cargo run` in your terminal and receive the default `Hello, World!`
print in your
console.

## Creating an HTML file output

I like to consider this step a very basic MVP (minimum viable product). At the very least, our program should be able to
save an html string to an html file. Not super useful to the end-user at first, but creating baby steps that are
achievable helps me keep momentum.

The next step is to make it so our cli binary can output an HTML file that we can upload. To make things easy, we won't
worry about parsing our templates yet. We'll supply a string literal in the source of html code, and simply have our
program save that string to a file in an output directory.

We'll start by editing `~/cli/src/main.rs`. First, we'll add the HTML string output we want to save and have
the `println!` macro print that instead.

```rust
fn main() {
    let html = "\
<html>
    <head>
        <title>example generator</title>
    </head>
    <body>
        <h1>It works!</h1>
    </body>
</html>";
    println!("{}", html);
}
```

Nice. Next, we'll iterate with html file output to a directory we specify, enabling us to lazily upload a specific
directory later.

We'll also add some improvements, like replacing the `println!` macro that we don't need with proper logging. I like
doing this early in a project, so that I have access to logging as early as possible without much thought.

We're going to add these projects into `src/cli/Cargo.toml`, which is just personal preference. I like these packages
because they come with macros like `debug!` and `error!` which you can just call outright.

- [env_logger](https://github.com/rust-cli/env_logger/)
- [log](https://docs.rs/log/latest/log/)

```toml
[package]
name = "website-example"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
log = "0.4.0"
env_logger = "0.10.0"
```

Then, we'll take advantage of our new packages by initializing the logger, replacing `println!` with a debug statement,
and finally adding logic to:

1. create our output directory
2. create our empty html file in the output directory
3. write our html string into the empty file

```rust
use env_logger::init;
use log::debug;
use std::fs;
use std::fs::File;
use std::io::Write;

fn main() {
    env_logger::init();
    let html = "\
<html>
    <head>
        <title>example generator</title>
    </head>
    <body>
        <h1>It works!</h1>
    </body>
</html>";
    debug!("[GENERATING HTML] {}", html);

    let output_dir = "_out";
    let html_path = format!("{}/index.html", &output_dir);

    debug!("[CREATING FILES] {}", html_path);
    fs::create_dir_all(output_dir)
        .expect("[DIRECTORY CREATE ERROR] Could not create output directory");

    let mut file =
        File::create(html_path).expect("[FILE CREATION ERROR] Could not create html file");
    file.write_all(&html.as_bytes())
        .expect("[FILE WRITE ERROR] Could not write html to file");
}
```

## CI/CD using GitHub Actions

For this next part, I'm using [GitHub Actions](https://docs.github.com/en/actions) to compile the binary, run it, and
upload the output files to my S3 bucket that I'm using as a static file.

Because there's a million and one different ways to approach this part, I'm not going to go too much into depth. My
action is configured to do all the above, as well as a cache directory to save some times on non-binary related runs.

```yaml
name: build and upload website
on:
  release:
    types:
      - released
      - prereleased
  workflow_dispatch:

jobs:
  run:
    runs-on: ubuntu-latest
    env:
      AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
      AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
    steps:
      - uses: actions/checkout@v3
      - name: install rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
      - uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      - name: build website
        uses: actions-rs/cargo@v1
        with:
          command: run
          args: --package cli
      - name: upload
        uses: reggionick/s3-deploy@v3
        with:
          folder: _out
          bucket: ${{ secrets.AWS_S3_BUCKET }}
          bucket-region: ${{ secrets.AWS_REGION }}
          dist-id: ${{ secrets.AWS_CLOUDFRONT_DISTRIBUTION_ID }}
          invalidation: /*
          delete-removed: true
          private: true
          filesToInclude: ".*/*,*/*,**"
```

And that's it! Now, every time we do a (pre)release it will compile our binary and deploy it. That's useful for pinning
the binary itself, but this yaml also enables manual deployments for when you just have content updates. This way, you
don't need to create a new version for content updates.

That's it for this part. In the next part we'll run through adding handlebars and SASS to get prettier pages.
