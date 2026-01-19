---
title: "Creating a queue for video processing"
date: 2024-10-15
category: "tech"
series_key: "video-streaming-devlog"
series_pos: 1
summary: "Using Rust and Postgres, we'll create a queue system for our video processing pipeline"
---
In our [previous post](https://sneakycrow.dev/blog/2024-10-13-creating-a-small-video-streaming-service)
we created a function that can process a video into a stream, and some routes for serving those
static files. The problem with doing this within the API is it can take up a lot of resources.
Ideally, we want to offload this processing.

So, what we're going to do in this article is create a queue system that can run our jobs.
We'll create a basic queue with an initial job type of `ProcessRawVideo`, representing processing a video into our raw stream.
This queue system will use Postgres to manage jobs, and will take advantage of `SKIP LOCKED` when querying for jobs
to add some concurrency.

This is largely inspired by [Sylvain Kerkour's post on creating a job queue system in Rust](https://kerkour.com/rust-job-queue-with-postgresql),
but slightly modified for our purposes. We're mostly focusing on the high level of moving a
video processing job through our pipeline.

## creating the queue

Our queue is largely made of three core entities: the queue itself, the jobs it processes, and
the runners that execute the job. We want to to be able to push and pull jobs to our queue, and we want to be able
run several jobs concurrently.

We're going to use Postgres to store jobs to be processed. And when we query Postgres we'll take advantage of it's locking
mechanisms to have some concurrency.

### dependencies

We'll mostly be using sqlx for it's simplicity here, but this should be fairly interchangable with
whatever library you're using.

Here's our `Cargo.toml` dependencies. A lot of these are preference and not really required to make it work. Again, this is
largely an extension of [Sylvain Kerkour's tutorial on queues](https://kerkour.com/rust-job-queue-with-postgresql).

```toml Cargo.toml
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = [
    "runtime-tokio-rustls",
    "postgres",
    "chrono",
    "uuid",
    "json",
] }
chrono = "0.4"
uuid = { version = "1", features = ["serde", "v4"] }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
thiserror = "1"
anyhow = "1"
ulid = { version = "1", features = ["uuid"] }
futures = "0.3"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### db module

This module holds our function for creating a database connection pool. We'll create a connection for our workers
to use for updating the processing status and job queue in our database.

```rust db.rs
use sqlx::postgres::PgPool;

/// Function to establish a connection to the PostgreSQL database
pub async fn connect_to_database() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    tracing::debug!("Creating DB connection Pool");
    let pool = PgPool::connect(&database_url).await?;

    Ok(pool)
}
```

### migrations

We'll want to create two tables, our main one is for managing the jobs, but we also want to update our videos. In our videos
table, we want to track where the raw video is located, whether it's been processed, and then standard stuff like an ID and
timestamps.

Tip: You can create a top level folder called `migrations` and then use the `sqlx` cli to run them, with `sqlx migrate run`

For the queue table, which holds the scheduled jobs, we want to track a timestamp that allows us to schedule it, how many
failed attempts (if any) there are, it's current status, and then the message payload. Additionally, the standard ids and
timestamps.

We'll also add some indexs on the `status` and `scheduled_for` columns for faster querying, as most of the time we're going
to be querying for rows that are `Queued`.

```sql 0001_init_queue.sql
CREATE TABLE queue (
  id UUID PRIMARY KEY,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL,
  updated_at TIMESTAMP WITH TIME ZONE NOT NULL,

  scheduled_for TIMESTAMP WITH TIME ZONE NOT NULL,
  failed_attempts INT NOT NULL,
  status INT NOT NULL,
  message JSONB NOT NULL
);
CREATE INDEX index_queue_on_scheduled_for ON queue (scheduled_for);
CREATE INDEX index_queue_on_status ON queue (status);
```

Next, we need a table for storing `videos`. These two migrations don't need to be in any particular order,
or could be combined into one. I just like to isolate my migrations logic.

For our videos, we want to store the `raw_file_path` where the raw video was uploaded, a column `processed_file_path`
for representing where the processed `m3u8` playlist is, and a `processing_status`which maps to the
current status of processing. Additionally, standard id and timestamp columns.

```sql 0002_init_videos.sql
CREATE TABLE videos (
    id TEXT PRIMARY KEY,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,

    raw_file_path TEXT NOT NULL,
    processed_file_path TEXT,
    processing_status TEXT NOT NULL DEFAULT 'pending'
);
```

We can run those with `sqlx migrate run`, which will setup the tables in our connected database. Make sure your `DATABASE_URL`
environment variable is set right.

### error module

I like to create a high level error module for my applications to re-use across my workspace. This is optional, but largely
used in the examples below.

```rust error.rs
/// Various errors that our queue can have
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("Bad config: {0}")]
    BadConfig(String),
    #[error("Connecting to database: {0}")]
    ConnectingToDatabase(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Migrating database: {0}")]
    DatabaseMigration(String),
    #[error("Invalid job message: {0}")]
    InvalidJobMessage(String),
    #[error("Video Processing: {0}")]
    VideoProcessingError(String),
}
/// Convertion of queue from an sqlx error to this error
impl std::convert::From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Error::NotFound("row not found".into()),
            _ => Error::Internal(err.to_string()),
        }
    }
}
```

### queue module

#### queue trait and job definitions

Now we enter the more meaty parts of this article. But, before we define the queue, which will hold jobs,
we need to define what a `Job` is. A job should have a unique identifier and some kind of message with a
payload to run it. For now, we only need a `ProcessRawVideo` payload, but we'll have this be a member of an
enum representing all payloads called `Message`.

Our specific message for processing a video will accept a `video_id` and a `path`. The path should be a path to the
raw video. We included our path as a column on our `videos` table, but including it here saves us a query. Then
we can use the `video_id` to update the row after we're done processing.

```rust lib.rs
/// The job to be processed, containing the message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub message: Message,
}

/// The payload of the job, containing the different jobs and their required data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Our primary processing job
    /// Takes in a raw video (mp4) and converts it into an HLS stream
    // NOTE: We intentionally have the user provide the path to save us a db query to get it, but we
    // need a db query after processing to update the job status
    ProcessRawVideo { path: String, video_id: String },
}
```

Now, we can define our queue. We're going to define a trait that's our base queue,
and then implement that trait into a `PostgresQueue`. By having the queue itself be a trait we could
use different configurations, like if we moved to a different backend for managing the queue parts.

Our queue needs to be able to push jobs, pull jobs, fails jobs, delete jobs, and clear the queue. All work together
to manage the queue itself.

```rust lib.rs
#[async_trait::async_trait]
pub trait Queue: Send + Sync + Debug {
    /// pushes a job to the queue
    async fn push(
        &self,
        job: Message,
        scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<(), Error>;
    /// pull fetches at most `number_of_jobs` from the queue.
    async fn pull(&self, number_of_jobs: i32) -> Result<Vec<Job>, Error>;
    /// deletes a job from the queue
    async fn delete_job(&self, job_id: Uuid) -> Result<(), Error>;
    /// fails a job in the queue
    async fn fail_job(&self, job_id: Uuid) -> Result<(), Error>;
    /// clears the queue
    async fn clear(&self) -> Result<(), Error>;
}
```

#### postgres queue impl

Next, we're going to implement our new queue trait into a `PostgresQueue`, which, as you might guess, is a queue
for Postgres (yeeeeehaw).

First, we'll create our basic queue structs and implement a function for spawning one.

```rust lib.rs
/// A queue that uses Postgres as it's backend
#[derive(Debug, Clone)]
pub struct PostgresQueue {
    db: PgPool,
    max_attempts: u32,
}
/// Implementation of Postgres-based queue
impl PostgresQueue {
    pub fn new(db: PgPool) -> PostgresQueue {
        let queue = PostgresQueue {
            db,
            max_attempts: 5,
        };

        queue
    }
}
```

There's not too much to this. `5` is an preferential number for myself, you can adjust this safely
to whatever you want for your system. Aside from that we just need to make sure we have a pool connection to utilize

Next, we are going to get into the **beef** 🥩. Admittedly, there's not a lot of fancy stuff happening here.

- For the pull, we need to pull jobs who's status is `Queued`, and set it as `Running` when we return it. It's important
to do the update as we pull it so it gets locked appropriately. Finally, we'll use `SKIP LOCKED` to skip any locked rows,
which is how we achieve our concurrency
- For the push, we just need to create a new row who's status is `Queued`
- Delete, fail, and clear are pretty self-explanatory

We'll start with `delete`, `clear`, and `fail` since they're pretty simple:

```rust
#[async_trait::async_trait]
impl Queue for PostgresQueue {
    /// Delete an item from the queue based on it's job ID
    async fn delete_job(&self, job_id: Uuid) -> Result<(), Error> {
        let query = "DELETE FROM queue WHERE id = $1";

        sqlx::query(query).bind(job_id).execute(&self.db).await?;
        Ok(())
    }
    /// Fail the job based on it's ID, increments the failed attempts by 1
    async fn fail_job(&self, job_id: Uuid) -> Result<(), Error> {
        let now = chrono::Utc::now();
        let query = "UPDATE queue
            SET status = $1, updated_at = $2, failed_attempts = failed_attempts + 1
            WHERE id = $3";

        sqlx::query(query)
            .bind(PostgresJobStatus::Queued)
            .bind(now)
            .bind(job_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }
    /// Clear the entire queue of all jobs (delete all rows in the table)
    async fn clear(&self) -> Result<(), Error> {
        let query = "DELETE FROM queue";

        sqlx::query(query).execute(&self.db).await?;
        Ok(())
    }
}
```

Not much too those, now we can implement our push and pull methods to bring it all together.

First, we'll do the `push` method. This method should:
- Create a unique ID for the job
- Calculate the scheduled date
- Wrap the message payload in JSON
- Insert all the above into a new row in the `queue` table

Here's how that looks:

```rust lib.rs
// ...omitted for brevity
/// Push a new job with the included message payload to the queue storage
async fn push(
    &self,
    job: Message,
    date: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<(), Error> {
    let scheduled_for = date.unwrap_or(chrono::Utc::now());
    let failed_attempts: i32 = 0;
    let message = Json(job);
    let status = PostgresJobStatus::Queued;
    let now = chrono::Utc::now();
    let job_id: Uuid = Ulid::new().into();
    let query = "INSERT INTO queue
        (id, created_at, updated_at, scheduled_for, failed_attempts, status, message)
        VALUES ($1, $2, $3, $4, $5, $6, $7)";

    sqlx::query(query)
        .bind(job_id)
        .bind(now)
        .bind(now)
        .bind(scheduled_for)
        .bind(failed_attempts)
        .bind(status)
        .bind(message)
        .execute(&self.db)
        .await?;
    Ok(())
}
// ... omitted for brevity
```

You're going to find a pattern to this all: It's actually not too bad complexity-wise. The design of this implementation
mainly relies on Postgres to do the complicated locking and unlocking of jobs. Which is great, because that let's us largely
focus on our application!

Lastly, but not least, the `pull` method. This uses a maximum limit of 100 jobs. This is an artifact of [the original](https://kerkour.com/rust-job-queue-with-postgresql)
implementation, but I think it's a solid number to cap at. I imagine you can increase this limit, you'd
be surprised what you can do with one database.

What `pull` needs to do is, based on the number of jobs provided (with a max of 100) is
query the database and update each returned item's status to `Running`. We'll also have it skip locked rows, so we can
run multiple queues if we wanted (concurrency). We'll also want to limit the jobs to not pulled any failed ones over our
maximum retry rate. Here's how it all looks put together:

```rust lib.rs
// ... omitted for brevity
/// Pulls <number_of_jobs> from the queue (maximum 100)
/// This updates all jobs pulled into a `Running` status
async fn pull(&self, number_of_jobs: i32) -> Result<Vec<Job>, Error> {
    let number_of_jobs = if number_of_jobs > 100 {
        100
    } else {
        number_of_jobs
    };
    let now = chrono::Utc::now();
    let query = "UPDATE queue
        SET status = $1, updated_at = $2
        WHERE id IN (
            SELECT id
            FROM queue
            WHERE status = $3 AND scheduled_for <= $4 AND failed_attempts < $5
            ORDER BY scheduled_for
            FOR UPDATE SKIP LOCKED
            LIMIT $6
        )
        RETURNING *";

    let jobs: Vec<PostgresJob> = sqlx::query_as::<_, PostgresJob>(query)
        .bind(PostgresJobStatus::Running)
        .bind(now)
        .bind(PostgresJobStatus::Queued)
        .bind(now)
        .bind(self.max_attempts as i32)
        .bind(number_of_jobs)
        .fetch_all(&self.db)
        .await?;
    Ok(jobs.into_iter().map(Into::into).collect())
}
// ... omitted for brevity
```

Great, our queue is made. All that's left is to create a function for running our video processor, and then wrap it all together.

#### runner module

This module defines our functions for running our jobs. We'll define one function that will spawn `number_of_jobs` and then concurrently pass each one
into a handler that will do the actual processing.

First, our runner that will pull our jobs in and pass them to handlers concurrently

```rust runner.rs
/// Runs a loop that pulls jobs from the queue and runs <concurrency> jobs each loop
pub async fn run_worker(queue: Arc<dyn Queue>, concurrency: usize, db_conn: &Pool<Postgres>) {
    loop {
        // Pulls jobs from the queue
        let jobs = match queue.pull(concurrency as i32).await {
            Ok(jobs) => jobs,
            Err(err) => {
                // Trace the error
                tracing::error!("runner: error pulling jobs {}", err);
                // Go to sleep and try again
                tokio::time::sleep(Duration::from_millis(500)).await;
                Vec::new()
            }
        };
        // Just for debugging the amount of jobs a queue has pulled in
        let number_of_jobs = jobs.len();
        if number_of_jobs > 0 {
            tracing::debug!("Fetched {} jobs", number_of_jobs);
        }
        // Run each jobs concurrently
        stream::iter(jobs)
            .for_each_concurrent(concurrency, |job| async {
                tracing::debug!("Starting job {}", job.id);
                let job_id = job.id;

                let res = match handle_job(job, db_conn).await {
                    Ok(_) => queue.delete_job(job_id).await,
                    Err(err) => {
                        println!("run_worker: handling job({}): {}", job_id, &err);
                        queue.fail_job(job_id).await
                    }
                };

                match res {
                    Ok(_) => {}
                    Err(err) => {
                        println!("run_worker: deleting / failing job: {}", &err);
                    }
                }
            })
            .await;
        // Take a break for a bit, we don't need to run every moment (our jobs are unlikely to complete that quickly)
        tokio::time::sleep(Duration::from_millis(125)).await;
    }
}
```

And then, a slightly updated version of our ffmpeg function from the
[last post](https://sneakycrow.dev/blog/2024-10-13-creating-a-small-video-streaming-service). Mostly just wrapped
in the Job itself.

```rust runner.rs
/// Individually processes a single job, based on its Job message type
async fn handle_job(job: Job, db: &Pool<Postgres>) -> Result<(), Error> {
    match job.message {
        // TODO: If you want to do other kinds of processing, you can define their jobs here
        // Process Raw Videos into HLS streams
        message @ Message::ProcessRawVideo { .. } => {
            tracing::debug!("Processing raw video: {:?}", &message);
            // Get the required data to parse the video
            let (input_path, video_id) = match &message {
                Message::ProcessRawVideo { path, video_id } => (path, video_id),
            };
            tracing::debug!(
                "Processing video: input_path={}, video_id={}",
                input_path,
                video_id
            );
            // Create our HLS stream from the mp4
            let output_path = format!("{}.m3u8", input_path.trim_end_matches(".mp4"));
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
                .output()
                .map_err(|e| Error::VideoProcessingError(e.to_string()))?;

            if !output.status.success() {
                tracing::error!("Error processing video into hls");
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(Error::VideoProcessingError(error.to_string()));
            }
            // Update the video ID status
            sqlx::query("UPDATE videos SET processing_status = 'processed' WHERE id = $1")
                .bind(&video_id)
                .execute(db)
                .await
                .map_err(|e| Error::VideoProcessingError(e.to_string()))?;
            tracing::debug!("Successfully processed video {}", &video_id);
        }
    };

    Ok(())
}
```

And that's our entire library for the queue! We can now create our queues! Yeehaw

### Finalizing

Finally, we run our queue. So, the way this works in a manner similar to a pub/sub model is that we can have many producers (push to the queue)
and a single consumer (pull from the queue). That doesn't mean we can run many queues that pull, it just means one job should only
ever go to one of those queue instances, but many producers can push new jobs to the queue.

Now, there's a lot of ways you can have these queues communicate. But the most straight-forward way is to run one somewhere for pushing events,
and one run somewhere for pulling events. For example, I create a queue instance in my API and allow it to push items to the queue, then I have a separate binary
running the queue as a worker and running the jobs.

Here's a simple binary that will run our jobs. It spawns an async task for running the queue worker. This is what would act as
a consumer to our queue.

```rust main.rs
const CONCURRENCY: usize = 5;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Start the tracer
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "event_processor=debug,db=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    // Create a database connection
    let db = db::connect_to_database().await?;
    // Initialize a queue
    tracing::debug!("Initializing queue");
    let queue = Arc::new(PostgresQueue::new(db.clone()));
    // Pass our queue (shared ref) to our runner
    let worker_queue = queue.clone();
    tokio::spawn(async move { run_worker(worker_queue, CONCURRENCY, &db).await });
    Ok(())
}
```

And for the producer, it's even simpler. Just spawn a queue (make sure it's pointed at the same database) and push
```rust main.rs
const CONCURRENCY: usize = 5;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Start the tracer
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "event_processor=debug,db=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    // Create a database connection
    let db = db::connect_to_database().await?;
    // Initialize a queue
    tracing::debug!("Initializing queue");
    let queue = PostgresQueue::new(db.clone());
    // Spawn a message and push it to our queue
    let path = "some/path/to/an/file.mp4".to_string();
    let video_id = "some_id_we_already_created".to_string();
    let job = Message::ProcessRawVideo { path, video_id };
    queue
        .push(job, None) /// If you want to schedule your job for later replace `None`
        .await
        .expect("Could not push job to queue");
    Ok(())
}
```

And that's a wrap! We now have a queue that we can push jobs too, backed by postgres, and a way to run those jobs on a
separate process. Next up we'll create a way to `LISTEN` to updates on the status of a video processing job so we can update
our clients when it's done.
