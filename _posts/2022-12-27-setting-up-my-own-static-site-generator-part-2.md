---
title: "Custom Static Site Generator - handlebars & SCSS"
date: 2022-12-27
category: "tech"
series_key: "website-generator-tutorial"
series_pos: 1
summary: "The second part of setting up a static site generator, setting up initial templating"
---

[In the first part of this series][part 1] we did some basic initial research, and then a "kinda-mvp" of a html file
generator. We also added a Github Actions config for auto-deployment.

In this part, we're going to create our templating system using [handlebars-rust][handlebars-rust]. Afterwards, we'll
insert some CSS using SCSS for styling. I'm using [grass][grass] to compile our SCSS.

I have no particularly reason that I decided on these two libraries beyond I'm familiar with handlebars and that I
wanted a simple lib for compiling scss. The reason we're using templating here is mainly for our blog posts, but we'll
take advantage of it for pages too.

Our goal for this part of the series is to generate a stylized index.html from a handlebars template.

First, we create our templates directory in the root directory. The reason we're doing it in the root directory is
because when rust is parsing relative files it starts there. It basically just makes referring to files clean and easy.

In addition, we know we're going to have SCSS files as well, so let's call the top level folder `assets` with two
subfolders `assets/templates` and `assets/scss`.

_note, you may notice I re-organized this [website repository](https://github.com/sneakycrow/website)_ to match this _
after_ I wrote this article. Shut up, software is iterative. I realized this was better when writing this, lol.

So, our repo should look something like this now:

```markdown
├── assets
│ ├── scss
│ ├── templates
├── cli
│ ├── src
│ └── Cargo.toml
├── Cargo.lock
├── Cargo.toml
```

Next, we'll add an `index.hbs` file in our templates directory, for our homepage. Since we expect to have multiple
pages, (like an about page for example), we want to create the directory `assets/templates/pages` as well.
For the homepage template, we'll basically just take the string we had in stored in the source code
from [part 1][part 1] and move it to the template file. So, we should
have the file `assets/templates/pages/index.hbs` now, and it should look something like this:

```handlebars
<html lang="en">
  <head>
    <title>{{title}}</title>
  </head>
  <body>
    <h1>It works!</h1>
  </body>
</html>
```

Next, we need to update our Rust code and introduce handlebars. For handlebars, you create a registry, load templates
into that registry, and then use that registry to compile your html. Let's add this logic into our `cli/src/main.rs`.

```rust
fn main() {
    env_logger::init();
    // Create handlebars registry
    let mut hbs_registry = Handlebars::new();
    // Load page templates directory
    hbs_registry
        .register_template_directory(".hbs", "assets/templates/pages")
        .expect("[HANDLEBARS ERROR] Could not register templates in assets/templates/pages");
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

Great...except it isn't actually using our template yet! Let's fix that. Let's update our template to use a
dynamic `<title>`, so we can validate that the template isn't just spitting itself out. Then we'll render the template
with a dynamic title we pass via `Serialize` struct.

```rust
fn main() {
    env_logger::init();
    // Create handlebars registry
    let mut hbs_registry = Handlebars::new();
    // Load page templates directory
    hbs_registry
        .register_template_directory(".hbs", "assets/templates/pages")
        .expect("[HANDLEBARS ERROR] Could not register templates in assets/templates/pages");
    let output_dir = "_out";
    let html_path = format!("{}/index.html", &output_dir);

    debug!("[CREATING FILES] {}", html_path);
    // Generate output directory
    fs::create_dir_all(output_dir)
        .expect("[DIRECTORY CREATE ERROR] Could not create output directory");

    // Render template with data
    let data = HashMap::from([("title", "example title")]);
    // Use our registry to render the template with our data
    let html = hbs_registry.render("index", &data).expect("Could not generate html");


    let mut file =
        File::create(html_path).expect("[FILE CREATION ERROR] Could not create html file");
    file.write_all(&html.as_bytes())
        .expect("[FILE WRITE ERROR] Could not write html to file");
}
```

After running this, it should generate an html page, using our template, with a `<title>` of the value `example title`.

Next, we'll introduce SCSS compilation via the [grass][grass] package. First, we'll want to create a `.scss` file in
our `assets/scss` directory. We'll start by adjusting the contrast of the default black/white to a dark gray/beige. You
don't have to do this, but you'll want to add something so you know it's working.

I'm creating a file `assets/scss/_index.scss` where I'll set some variables and make a few basic changes.

```scss
$black: #1f1f1f;
$white: #efefef;

body {
  background-color: $white;
  color: $black;
}
```

Next, we'll update our Rust code to read the SCSS in and spit CSS out, which we then save into a `.css` file.

```rust
fn main() {
    env_logger::init();
    // Create handlebars registry
    let mut hbs_registry = Handlebars::new();
    // Load page templates directory
    hbs_registry
        .register_template_directory(".hbs", "assets/templates/pages")
        .expect("[HANDLEBARS ERROR] Could not register templates in assets/templates/pages");
    let output_dir = "_out";
    let html_path = format!("{}/index.html", &output_dir);

    debug!("[CREATING FILES] {}", html_path);
    // Generate output directory
    fs::create_dir_all(output_dir)
        .expect("[DIRECTORY CREATE ERROR] Could not create output directory");

    // Render template with data
    let data = HashMap::from([("title", "example title")]);
    // Use our registry to render the template with our data
    let html = hbs_registry.render("index", &data).expect("Could not generate html");


    let mut file =
        File::create(html_path).expect("[FILE CREATION ERROR] Could not create html file");
    file.write_all(&html.as_bytes())
        .expect("[FILE WRITE ERROR] Could not write html to file");

    let css = grass::from_path("assets/scss/_index.scss", &grass::Options::default()).expect(
        format!(
            "[SASS COMPILATION ERROR] Could not compile sass file {}",
            path
        )
            .as_str(),
    );
    let css_path = format!("{}/main.css", &output_dir);
    let mut css_file =
        File::create(css_path).expect("[FILE CREATION ERROR] Could not create html file");
    file.write_all(&css.as_bytes())
        .expect("[FILE WRITE ERROR] Could not write html to file");
}
```

Lastly, make sure to add the appropriate `<link>` to our html template

```handlebars
<html lang="en">
  <head>
    <title>{{title}}</title>
    <link rel="stylesheet" href="/main.css" />
  </head>
  <body>
    <h1>It works!</h1>
  </body>
</html>
```

Congratulations! You should now have a templated static site generator with handlebars and SCSS! In the next tutorial
we'll implement a markdown parser so we can write static blog posts in markdown.

[part 1]: https://sneakycrow.dev/2022/11/26/setting-up-my-own-static-site-generator.html
[handlebars-rust]: https://crates.io/crates/handlebars
[grass]: https://crates.io/crates/grass
