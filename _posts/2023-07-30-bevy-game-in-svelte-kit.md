---
title: "Running a Bevy game in SvelteKit"
date: 2023-07-30
slug: "bevy-game-in-svelte-kit"
summary: "Running your bevy games with wasm in SvelteKit"
category: "tech"
---

Recently, I started working on a new project using [Bevy](https://bevyengine.org/), a game engine written in Rust. I've
been
really enjoying it so far, and I'm excited to see where it goes.

I'd like to document the process of developing games, but I also wanted people to be able to play wasm builds of the
games I make.
[I recently converted my entire website to SvelteKit](https://github.com/sneakycrow/website/releases/tag/v6.0.0),
and I wanted to be able to import my games into it. I couldn't find any examples of using Bevy with SvelteKit
specifically, so I thought I'd write one.

For this tutorial we will create a basic SvelteKit project, create a Rust workspace with a Bevy game, and configure
SvelteKit to run the game.

## Creating a basic SvelteKit project

We'll start with creating a basic SvelteKit project. If you already have a project you'd like to use, you can skip this

```shell
npm create svelte@latest my-sveltekit-project && cd sveltekit-project
```

Next, we'll need to update the Vite configuration to be able to import wasm later.
We're going to utilize the [vite-plugin-wasm]() and [vite-plugin-top-level-await]() plugins to do this.

```shell
npm install -D vite-plugin-wasm vite-plugin-top-level-await
```

Next, open `vite.config.js` and update the `plugins` section to look like this:

```js vite.config.js
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";
import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  plugins: [sveltekit(), wasm(), topLevelAwait()]
});
```

Before we can import wasm, we need...the wasm. So next, we'll initialize our Bevy game.

## Adding a Bevy game

We'll start by creating a new Rust workspace. If you already have a workspace you'd like to use, you can skip this step.

Add a new file called `Cargo.toml` to the root of your project with the following contents:

```toml
[workspace]
members = ["game"]
```

Next, we'll initialize our bevy game in the `game` directory.

```shell
cargo new game
```

You'll need to also make sure you have the wasm target installed. You can do this by running:

```shell
rustup target add wasm32-unknown-unknown
```

Next, add [bevy](https://bevyengine.org) as a dependency in our `game/Cargo.toml` file.

```toml
[dependencies]
bevy = "0.11"
```

For this next part, any game that can compile to wasm should be fine.

One important change you'll want to make though is to update the WindowPlugin canvas property to use a custom id. This
is how we're going to control the canvas element from SvelteKit.

Here's an example of customizing the WindowPlugin:

```rust
use bevy::prelude::*;

fn main() {
    let mut window_plugin = WindowPlugin::default();
    window_plugin.primary_window = Some(Window {
        title: "Game".to_string(),
        canvas: Some("#game_canvas".to_string()), // This is the important part
        ..Default::default()
    });
    let plugins = DefaultPlugins.set(window_plugin);
    App::new()
        .add_plugins(plugins)
        .run();
}

```

If you'd like a basic game to test with, you can
use the following example [from bevy](https://github.com/bevyengine/bevy/blob/latest/examples/2d/2d_shapes.rs):

```rust
//! Shows how to render simple primitive shapes with a single color.

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};

fn main() {
    let mut window_plugin = WindowPlugin::default();
    window_plugin.primary_window = Some(Window {
        title: "Game".to_string(),
        canvas: Some("#game_canvas".to_string()),
        ..Default::default()
    });
    let plugins = DefaultPlugins.set(window_plugin);
    App::new()
        .add_plugins(plugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    // Circle
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(shape::Circle::new(50.).into()).into(),
        material: materials.add(ColorMaterial::from(Color::PURPLE)),
        transform: Transform::from_translation(Vec3::new(-150., 0., 0.)),
        ..default()
    });

    // Rectangle
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0.25, 0.25, 0.75),
            custom_size: Some(Vec2::new(50.0, 100.0)),
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(-50., 0., 0.)),
        ..default()
    });

    // Quad
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(shape::Quad::new(Vec2::new(50., 100.)).into())
            .into(),
        material: materials.add(ColorMaterial::from(Color::LIME_GREEN)),
        transform: Transform::from_translation(Vec3::new(50., 0., 0.)),
        ..default()
    });

    // Hexagon
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(shape::RegularPolygon::new(50., 6).into()).into(),
        material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
        transform: Transform::from_translation(Vec3::new(150., 0., 0.)),
        ..default()
    });
}
```

Next, we just need to build our game for the wasm target.

```shell
cargo build --release --bin game --target wasm32-unknown-unknown
```

Now we should have a wasm file at `target/wasm32-unknown-unknown/release/game.wasm`.

We're almost done! Next we're going to utilize [wasm-bindgen](https://github.com/rustwasm/wasm-bindgen) to
package our wasm file into a js/ts file that we can import into our SvelteKit project.

## Generating lib files with wasm-bindgen

We're going to run wasm-bindgen and have it output our files to an `assets` directory for Svelte to be able to import.

```shell
wasm-bindgen --out-dir src/assets/game --target web ./target/wasm32-unknown-unknown/release/game.wasm
```

This will generate a `game.js` and `game_bg.wasm` file in the `src/assets/game` directory.

## Configuring SvelteKit to run the game

Now that we have importable js files, we can create our Svelte component to run the game.
We'll create a component that renders a canvas element with a matching id to the one we specified in our game. And then
we can customize this canvas component

Create a new file `src/lib/components/Game.svelte` with the following contents:

```svelte
<script>
  import { onMount } from "svelte";
  import __wbg_init from "../../assets/game";

  onMount(async () => {
    await __wbg_init();
  });
</script>

<canvas id="game_canvas" />
```

Now we can import this component into our `src/routes/index.svelte` file and render it.

```svelte
<script>
  import Game from "$lib/components/Game.svelte";
</script>

<Game />
```

Now we should be able to run our SvelteKit project and see our game running!

```shell
npm run dev
```
