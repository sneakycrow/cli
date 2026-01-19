---
title: "Creating a small video streaming service"
date: 2024-10-13
category: "tech"
series_key: "video-streaming-devlog"
series_pos: 0
summary: "a devlog for how to create a small video streaming service"
---
## intro
I've been working on a personal file sharing project, and one feature I want to add to this project is video streaming for
large videos. I wanted a place where I could keep my stream replays but that had reasonable cost.
Serving large 4+ hour videos can be very costly if you're not streaming it, as you may imagine.

So, what I wanted to do was enable a way for me to upload a raw video, and then have the server convert that
into an [hls](https://en.wikipedia.org/wiki/HTTP_Live_Streaming) stream that we can stream back to the client.
This is a lot more cost-efficient than trying to stream a huge raw video back to the client.

One problem is that I'm using a simple [axum](https://github.com/tokio-rs/axum) server, and I don't want the API to
be overwhelmed by trying to process videos in the same upload request. So, we'll create a second server that's
exclusively for processing videos. That allows the API and the video processor to scale separately of each other. But,
we'll also need a way to trigger processing jobs. We'll build a queue using Postgres and it's SKIP LOCKED mechanism
for some concurrency.

One UX issue: The user can't watch the video until it's hls stream is ready, but the processing doesn't happen in the
upload request. So, what we'll need to do is give the ability for the user to subscribe to updates about their
video processing status. We can solve this using Postgres listen/notify events with a websocket endpoint on our API.

All in all, this dev log will go through these parts:
1. creating a video processor for converting a raw video into an hls stream using [ffmpeg](https://ffmpeg.org/)
2. creating a queue system using Postgres
3. creating a video processing server using gRPC
4. creating a websocket endpoint for listening to processing updates

Additionally, we'll create some UI in all the appropriate parts

## convert raw video to an hls stream

This is probably the simplest part of this whole series, as we're going to take advantage of [ffmpeg](https://ffmpeg.org/)
to do most of the heavy lifting for us. We'll just create a Rust binary that calls ffmpeg externally.

__Note: There are [crates that provide ffmpeg bindings](https://github.com/zmwangx/rust-ffmpeg?tab=readme-ov-file), but we
don't really need to do anything other than making a fairly standard ffmpeg call_

There's not much to this, it just uses the standard library and expects `ffmpeg` to be available on the host

```rust
async fn process_video() {
    let input_path = "some_path/to/video.mp4";
    let output_path = "wherever_you/want/the/stream_files/"
    let output = std::process::Command::new("ffmpeg")
        .args(&[
            "-i",
            input_path,
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
            "-f",
            "hls",
            "-hls_time",
            "10",
            "-hls_list_size",
            "0",
            &output_path,
        ])
        // This ffmpeg command:
        // - Takes an input video file specified by `input_path`
        // - Transcodes the video to H.264 codec (-c:v libx264)
        // - Transcodes the audio to AAC codec (-c:a aac)
        // - Outputs an HLS stream (-f hls)
        // - Sets the duration of each segment to 10 seconds (-hls_time 10)
        // - Creates an unlimited number of segments in the playlist (-hls_list_size 0)
        // - Saves the output to the path specified by `output_path`
        .output()
        // TODO: Make this better error handling
        .expect("Could not process video into HLS stream");
}
```

When modifying the inputs/outputs and running this, you should see some `.m3u8` and `.ts` files get generated in our
output directory. We can serve these files on the frontend using [hls.js](https://github.com/video-dev/hls.js/)

This HTML snippet is from an askama template I made. It renders with a `video_id` and `stream_url`. The `stream_url`
specifically points to the endpoint in my axum server that serves the the hls stream files.

```html
<section>
    <video id="{{ video_id }}" controls height="600"></video>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/hls.js/0.5.14/hls.min.js"></script>
    <script>
        var video = document.getElementById("{{ video_id }}");
        if (Hls.isSupported()) {
            var hls = new Hls({
                debug: true,
            });
            hls.loadSource("{{ stream_url }}");
            hls.attachMedia(video);
            hls.on(Hls.Events.MEDIA_ATTACHED, function () {
                video.muted = true;
                video.play();
            });
        }
        // hls.js is not supported on platforms that do not have Media Source Extensions (MSE) enabled.
        // When the browser has built-in HLS support (check using `canPlayType`), we can provide an HLS manifest (i.e. .m3u8 URL) directly to the video element through the `src` property.
        // This is using the built-in support of the plain video element, without using hls.js.
        else if (video.canPlayType("application/vnd.apple.mpegurl")) {
            video.src = "{{ stream_url }}";
            video.addEventListener("canplay", function () {
                video.play();
            });
        }
    </script>
</section>
```

In my axum server, I just serve the directory the hls files live in
```rust
// ... inside the Router
.nest_service("/uploads", ServeDir::new(uploads_path))
```

If you want to serve specific videos, I recommend saving your playlist files (`.m3u8`) with some kind of video ID.
Then you can create a fairly simple handler that serves those specific files. Here's a handler that renders the template above
with the appropriate variables.
```rust
pub async fn handle_stream_video(Query(params): Query<WatchQuery>) -> WatchTemplate {
    WatchTemplate {
        video_id: params.v.clone(),
        stream_url: format!("/uploads/{}.m3u8", params.v),
    }
}
```

And that's basically it for this part. In the next posts we'll work this functionality into our video processing server to refine
it a bit more. Maybe handling errors better?


Until the next episode...

---
If you want a more complete example, check out the source code for my [file nest](https://github.com/sneakycrow/file-nest)
project. It's where I'm using a lot of this functionality.
