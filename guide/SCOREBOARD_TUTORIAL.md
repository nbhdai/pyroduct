# Building a Live Scoreboard with the Daemon Client

This tutorial walks you through building a refreshing terminal scoreboard that
pulls scores from one or more playbooks, ranks them, and prints a live
leaderboard — all using `DaemonClient`.

---

## What We're Building

```
╔══════════════════════════════════════════════╗
║          🏆  LEADERBOARD  🏆                 ║
╠════╦═══════════════════════╦════════╦════════╣
║ #  ║ Name                  ║ Score  ║ Delta  ║
╠════╬═══════════════════════╬════════╬════════╣
║  1 ║ alice                 ║  9 420 ║  +310  ║
║  2 ║ bob                   ║  8 100 ║   +50  ║
║  3 ║ carol                 ║  7 880 ║  -120  ║
╚════╩═══════════════════════╩════════╩════════╝
   Playbook: game_scoring   Refreshed: 14:31:05
```

The scoreboard works by:

1. Connecting to the running daemon once.
2. Sending a SQL query against a playbook's output data (WAL + Parquet).
3. Decoding the Arrow IPC response into rows.
4. Sorting by score and rendering a ranked table.
5. Repeating on an interval for a live view.

---

## Prerequisites

- A playbook is running (or has run) whose output schema includes at least a
  `name : Utf8` and a `score : Int64` column (adjust column names to your schema).
- The daemon is up and its socket is accessible.

---

## The Wire Protocol

Every message to and from the daemon is a **16-byte binary header** followed
immediately by a **raw JSON payload** (the payload length is stored in the
header). There is no HTTP or other envelope — you can speak it from any
language that can open a Unix domain socket or TCP connection.

### Frame layout

```
Offset  Size  Field        Notes
──────────────────────────────────────────────────────────
 0x00    4    length       u32 LE — byte length of the JSON payload that follows
 0x04    4    client_id    u32 LE — set to 0 for a plain client
 0x08    1    wire_format  always 1  (PROTOCOL_VERSION)
 0x09    1    status       4 = JsonValid for requests; read from response
 0x0A    1    class_id     0 for daemon RPC
 0x0B    1    fn_id        0 for daemon RPC
 0x0C    4    mux_id       u32 LE — correlation ID; echo it back in the response
──────────────────────────────────────────────────────────
[payload]    <length> bytes of UTF-8 JSON
```

Constants that matter:
- `wire_format = 1` — always.
- `status = 4` — `DataStatus::JsonValid` — tells the server the payload is JSON.
- `mux_id` — the server echoes this back in its response header. Use any nonzero
  `u32`; `1` is fine for a simple one-shot client.

### Request JSON shape

```json
{
  "type": "data",
  "action": "query_playbook",
  "playbook_name": "game_scoring",
  "sql_query": "SELECT player_name, SUM(points) AS score FROM data GROUP BY player_name ORDER BY score DESC LIMIT 20"
}
```

### Response JSON shape

```json
{ "type": "data", "status": "query_result", "ipc_bytes": [137, 65, 82, ...] }
```

The `ipc_bytes` array is a standard Arrow IPC file — decode it with any Arrow
library (Rust `arrow`, Python `pyarrow`, etc.).

---

## Querying with `DaemonClient`

The [wire protocol](#the-wire-protocol) above is handled for you by
`DaemonClient`. Connect once and call `query_playbook` — no manual framing
needed.

Add `pyro-daemon` from crates.io to your `Cargo.toml`:

```toml
[dependencies]
pyro-daemon = "0.2.5"
arrow       = "57"
tokio       = { version = "1", features = ["full"] }
anyhow      = "1"
```

```rust
use arrow::array::{Int64Array, StringArray};
use arrow::ipc::reader::FileReader;
use pyro_daemon::client::DaemonClient;

const PLAYBOOK: &str = "game_scoring";
const SQL: &str = "
    SELECT player_name, SUM(points) AS score
    FROM data
    GROUP BY player_name
    ORDER BY score DESC
    LIMIT 20
";

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScoreEntry {
    pub rank:  usize,
    pub name:  String,
    pub score: i64,
}

pub async fn fetch_scoreboard() -> anyhow::Result<Vec<ScoreEntry>> {
    // connect_default() resolves the socket path automatically
    let client = DaemonClient::connect_default().await?;

    let ipc_bytes = client
        .query_playbook(PLAYBOOK.to_string(), SQL.to_string())
        .await?;

    decode_scores(ipc_bytes)
}

fn decode_scores(ipc: Vec<u8>) -> anyhow::Result<Vec<ScoreEntry>> {
    if ipc.is_empty() {
        return Ok(vec![]);
    }
    let mut reader = FileReader::try_new(std::io::Cursor::new(ipc), None)?;
    let mut entries = Vec::new();

    while let Some(Ok(batch)) = reader.next() {
        let names  = batch.column_by_name("player_name")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| anyhow::anyhow!("Missing 'player_name' column"))?;
        let scores = batch.column_by_name("score")
            .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
            .ok_or_else(|| anyhow::anyhow!("Missing 'score' column"))?;

        for i in 0..batch.num_rows() {
            entries.push(ScoreEntry {
                rank:  0,
                name:  names.value(i).to_string(),
                score: scores.value(i),
            });
        }
    }

    entries.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    for (i, e) in entries.iter_mut().enumerate() {
        e.rank = i + 1;
    }
    Ok(entries)
}
```

> [!TIP]
> Use `DaemonClient::connect_tcp("127.0.0.1:9000")` if the daemon was started
> with `--bind-tcp`. Use `DaemonClient::connect("/path/to/control")` to specify
> an explicit socket path.

---

## Axum Scoreboard Server

Expose the scoreboard over HTTP as a JSON endpoint. Add Axum to your
`Cargo.toml`:

```toml
[dependencies]
pyro-daemon = "0.2.5"
arrow       = "57"
tokio       = { version = "1", features = ["full"] }
anyhow      = "1"
axum        = "0.7"
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
```

```rust
use axum::{Router, routing::get, Json, http::StatusCode, extract::State};
use pyro_daemon::client::DaemonClient;
use std::sync::Arc;

/// Shared daemon client — one connection reused across all requests.
/// DaemonClient is Clone and its underlying socket is already multiplexed,
/// so cloning is cheap and safe for concurrent use.
#[derive(Clone)]
struct AppState {
    client: Arc<DaemonClient>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = DaemonClient::connect_default().await?;
    let state  = AppState { client: Arc::new(client) };

    let app = Router::new()
        .route("/scoreboard", get(scoreboard_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Scoreboard listening on http://0.0.0.0:3000/scoreboard");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn scoreboard_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<ScoreEntry>>, (StatusCode, String)> {
    let ipc = state.client
        .query_playbook(PLAYBOOK.to_string(), SQL.to_string())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    decode_scores(ipc)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
```

The `/scoreboard` endpoint returns JSON like:

```json
[
  { "rank": 1, "name": "alice", "score": 9420 },
  { "rank": 2, "name": "bob",   "score": 8100 }
]
```

> [!NOTE]
> `DaemonClient` is `Clone` and internally multiplexes requests over a single
> connection, so you can share it directly via `Arc<DaemonClient>` without a
> `Mutex`. Each concurrent Axum request gets its own mux slot automatically.

---

## Using `DaemonClient` (terminal scoreboard)

If you are already in a Rust crate and prefer to skip the raw framing, use
the `pyro-daemon` client, which handles the protocol for you.

Add it to your binary's `Cargo.toml`:

```toml
[dependencies]
pyro-daemon = "0.2.5"
arrow       = "57"
tokio       = { version = "1", features = ["full"] }
anyhow      = "1"
chrono      = { version = "0.4", features = ["clock"] }
```

## Step 1 — Connect to the Daemon

`DaemonClient::connect_default()` resolves the control socket automatically
using the same priority chain as `pyro-daemond` itself:

| Priority | Location |
|---|---|
| 1 | `$PYRO_DAEMON_DIR` env var |
| 2 | `/var/lib/pyroduct/control` (Linux systemd) |
| 3 | `~/.pyroduct/control` (install.sh / macOS) |
| 4 | `~/Library/Application Support/pyro-daemon/control` (legacy macOS) |

```rust
use pyro_daemon::client::DaemonClient;

async fn connect() -> anyhow::Result<DaemonClient> {
    Ok(DaemonClient::connect_default().await?)
}
```

> [!TIP]
> Use `DaemonClient::connect_tcp("127.0.0.1:9000")` if the daemon was started
> with `--bind-tcp`. Use `DaemonClient::connect("/path/to/control")` to specify
> an explicit socket path.

---

## Step 2 — Query Score Data

The scoreboard SQL should return one row per entity. The table is always named
**`data`** inside DataFusion, matching your playbook's output schema:

```rust
const PLAYBOOK: &str = "game_scoring";

// Returns (player_name, score) pairs sorted highest-first
const SCORES_SQL: &str = "
    SELECT player_name, SUM(points) AS score
    FROM data
    GROUP BY player_name
    ORDER BY score DESC
    LIMIT 20
";
```

Call `query_playbook` to execute it:

```rust
async fn fetch_scores(client: &DaemonClient) -> anyhow::Result<Vec<u8>> {
    let ipc_bytes = client
        .query_playbook(PLAYBOOK.to_string(), SCORES_SQL.to_string())
        .await?;
    Ok(ipc_bytes)
}
```

> [!NOTE]
> `query_playbook` sends a [`DataRequest::QueryPlaybook`](file:///Users/sven/nbhd/pyroduct/lib/pyro-daemon/src/data/mod.rs#L20-L23)
> message over the socket. The daemon restores the playbook's `DataManager`
> (WAL + IPC + Parquet storage layers) and runs the SQL through DataFusion
> before returning Arrow IPC bytes. Stopped playbooks still have their
> historical data queryable this way.

---

## Step 3 — Decode Arrow IPC into Rows

The response is a standard Arrow IPC file. Decode it with the `arrow` crate:

```rust
use arrow::array::{Int64Array, StringArray};
use arrow::ipc::reader::FileReader;

#[derive(Debug, Clone)]
pub struct ScoreEntry {
    pub name:  String,
    pub score: i64,
}

fn decode_scores(ipc_bytes: Vec<u8>) -> anyhow::Result<Vec<ScoreEntry>> {
    if ipc_bytes.is_empty() {
        return Ok(vec![]);
    }

    let cursor = std::io::Cursor::new(ipc_bytes);
    let mut reader = FileReader::try_new(cursor, None)?;
    let mut entries = Vec::new();

    while let Some(batch_res) = reader.next() {
        let batch = batch_res?;

        let names = batch
            .column_by_name("player_name")   // ← match your schema
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| anyhow::anyhow!("Missing 'player_name' column"))?;

        let scores = batch
            .column_by_name("score")
            .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
            .ok_or_else(|| anyhow::anyhow!("Missing 'score' column"))?;

        for i in 0..batch.num_rows() {
            entries.push(ScoreEntry {
                name:  names.value(i).to_string(),
                score: scores.value(i),
            });
        }
    }

    // DataFusion ORDER BY already sorts, but belt-and-suspenders:
    entries.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    Ok(entries)
}
```

---

## Step 4 — Render the Leaderboard

A pure-`std` terminal renderer with rank, name, score, and delta from the
previous refresh:

```rust
fn render(entries: &[ScoreEntry], prev: &[ScoreEntry]) {
    // Move cursor to top-left and clear (Linux/macOS)
    print!("\x1B[2J\x1B[H");

    println!("╔══════════════════════════════════════════════╗");
    println!("║          🏆  LEADERBOARD  🏆                 ║");
    println!("╠════╦═══════════════════════╦════════╦════════╣");
    println!("║ #  ║ Name                  ║  Score ║  Delta ║");
    println!("╠════╬═══════════════════════╬════════╬════════╣");

    for (rank, entry) in entries.iter().enumerate() {
        let prev_score = prev
            .iter()
            .find(|e| e.name == entry.name)
            .map(|e| e.score)
            .unwrap_or(entry.score);

        let delta = entry.score - prev_score;
        let delta_str = if delta >= 0 {
            format!("+{delta}")
        } else {
            format!("{delta}")
        };

        println!(
            "║ {:>2} ║ {:<21} ║ {:>6} ║ {:>6} ║",
            rank + 1,
            truncate(&entry.name, 21),
            entry.score,
            delta_str,
        );
    }

    println!("╚════╩═══════════════════════╩════════╩════════╝");
    println!(
        "   Playbook: {}   Refreshed: {}",
        PLAYBOOK,
        chrono::Local::now().format("%H:%M:%S"),
    );
}

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        None => s,
        Some((i, _)) => &s[..i],
    }
}
```

---

## Step 5 — Poll on an Interval

Wrap everything in a `tokio::time::interval` loop to refresh automatically:

```rust
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client   = connect().await?;
    let mut tick = time::interval(Duration::from_secs(5));
    let mut prev: Vec<ScoreEntry> = vec![];

    loop {
        tick.tick().await;

        match fetch_scores(&client).await.and_then(decode_scores) {
            Ok(entries) => {
                render(&entries, &prev);
                prev = entries;
            }
            Err(e) => eprintln!("Error: {e}"),
        }
    }
}
```

---

## Complete Binary

Drop this into `src/bin/scoreboard.rs` and run with
`cargo run --bin scoreboard`:

```rust
use arrow::array::{Int64Array, StringArray};
use arrow::ipc::reader::FileReader;
use pyro_daemon::client::DaemonClient;
use std::time::Duration;
use tokio::time;

const PLAYBOOK: &str = "game_scoring";
const SQL: &str = "
    SELECT player_name, SUM(points) AS score
    FROM data
    GROUP BY player_name
    ORDER BY score DESC
    LIMIT 20
";

#[derive(Clone)]
struct Entry {
    name:  String,
    score: i64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = DaemonClient::connect_default().await?;
    let mut tick = time::interval(Duration::from_secs(5));
    let mut prev: Vec<Entry> = vec![];

    loop {
        tick.tick().await;

        let ipc = client
            .query_playbook(PLAYBOOK.to_string(), SQL.to_string())
            .await
            .unwrap_or_default();

        let mut entries = decode(ipc).unwrap_or_default();
        render(&entries, &prev);
        prev = entries;
    }
}

fn decode(ipc: Vec<u8>) -> anyhow::Result<Vec<Entry>> {
    if ipc.is_empty() {
        return Ok(vec![]);
    }
    let mut reader = FileReader::try_new(std::io::Cursor::new(ipc), None)?;
    let mut out = vec![];

    while let Some(Ok(batch)) = reader.next() {
        let names  = batch.column_by_name("player_name")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| anyhow::anyhow!("Missing player_name"))?;
        let scores = batch.column_by_name("score")
            .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
            .ok_or_else(|| anyhow::anyhow!("Missing score"))?;

        for i in 0..batch.num_rows() {
            out.push(Entry {
                name:  names.value(i).to_string(),
                score: scores.value(i),
            });
        }
    }

    out.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    Ok(out)
}

fn render(entries: &[Entry], prev: &[Entry]) {
    print!("\x1B[2J\x1B[H");
    println!("╔══════════════════════════════════════════════╗");
    println!("║          🏆  LEADERBOARD  🏆                 ║");
    println!("╠════╦═══════════════════════╦════════╦════════╣");
    println!("║ #  ║ Name                  ║  Score ║  Delta ║");
    println!("╠════╬═══════════════════════╬════════╬════════╣");

    for (i, e) in entries.iter().enumerate() {
        let prev_score = prev.iter().find(|p| p.name == e.name).map(|p| p.score).unwrap_or(e.score);
        let d = e.score - prev_score;
        let ds = if d >= 0 { format!("+{d}") } else { format!("{d}") };
        let name = &e.name[..e.name.len().min(21)];
        println!("║ {:>2} ║ {:<21} ║ {:>6} ║ {:>6} ║", i + 1, name, e.score, ds);
    }

    println!("╚════╩═══════════════════════╩════════╩════════╝");
    println!(
        "   Playbook: {}   Refreshed: {}",
        PLAYBOOK,
        chrono::Local::now().format("%H:%M:%S"),
    );
}
```

---

## Extensions

### Multi-Playbook Leaderboard

Discover running playbooks dynamically and fan out queries in parallel:

```rust
use futures::future::join_all;

let playbooks = client.list_playbooks().await?;

let results = join_all(playbooks.iter().map(|pb| {
    let c     = client.clone();
    let name  = pb.name.clone();
    async move {
        let ipc     = c.query_playbook(name.clone(), SQL.to_string()).await?;
        let entries = decode(ipc)?;
        anyhow::Ok((name, entries))
    }
}))
.await;
```

`list_playbooks` also returns `processed_rows` on each [`PlaybookStatus`](file:///Users/sven/nbhd/pyroduct/lib/pyro-daemon/src/playbook/manager.rs#L17-L29) —
useful as a cheap throughput metric without running SQL.

### Error-Rate Scoreboard

Use SQL directly for a rolling error rate:

```rust
const ERROR_SQL: &str = "
    SELECT
        COUNT(*) FILTER (WHERE status = 'error')                        AS errors,
        COUNT(*)                                                         AS total,
        CAST(COUNT(*) FILTER (WHERE status = 'error') AS DOUBLE)
            / NULLIF(COUNT(*), 0) * 100.0                               AS error_pct
    FROM data
    WHERE created_at > NOW() - INTERVAL '1 hour'
";
```

Or pull raw failure records with `get_playbook_failures` for a zero-SQL approach:

```rust
let failures = client
    .get_playbook_failures("my_playbook".to_string())
    .await?;
println!("{} recent failures", failures.len());
```

### Export to JSON each Cycle

```rust
let json = serde_json::to_string_pretty(
    &entries.iter().enumerate().map(|(i, e)| {
        serde_json::json!({ "rank": i + 1, "name": e.name, "score": e.score })
    }).collect::<Vec<_>>(),
)?;
std::fs::write("scoreboard.json", json)?;
```

---

## API Quick-Reference

| Method | Purpose |
|---|---|
| [`DaemonClient::connect(path)`](file:///Users/sven/nbhd/pyroduct/lib/pyro-daemon/src/client.rs#L16-L21) | Connect via Unix socket |
| [`DaemonClient::connect_tcp(addr)`](file:///Users/sven/nbhd/pyroduct/lib/pyro-daemon/src/client.rs#L29-L34) | Connect via TCP |
| [`DaemonClient::query_playbook`](file:///Users/sven/nbhd/pyroduct/lib/pyro-daemon/src/data/client.rs#L18-L32) | Run SQL → Arrow IPC bytes |
| [`DaemonClient::list_playbooks`](file:///Users/sven/nbhd/pyroduct/lib/pyro-daemon/src/playbook/client.rs#L64-L71) | List running playbooks + `processed_rows` |
| [`DaemonClient::get_playbook_failures`](file:///Users/sven/nbhd/pyroduct/lib/pyro-daemon/src/data/client.rs#L52-L62) | Recent raw failure records |
| [`DaemonClient::get_playbook_data`](file:///Users/sven/nbhd/pyroduct/lib/pyro-daemon/src/data/client.rs#L34-L50) | Paginated raw output rows |
| [`PyroDaemon::default_working_dir`](file:///Users/sven/nbhd/pyroduct/lib/pyro-daemon/src/lib.rs#L76-L110) | Resolve socket path automatically |
