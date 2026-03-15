use std::collections::HashMap;
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Transport errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportClosedError;

impl std::fmt::Display for TransportClosedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "transport channel closed")
    }
}

impl std::error::Error for TransportClosedError {}

// ---------------------------------------------------------------------------
// Wire-level envelope types  (internal — users never construct these)
// ---------------------------------------------------------------------------

enum Request<T> {
    ExecuteQuery     { request_id: u32, query: T },
    ExecuteStream    { request_id: u32, query: T },
    Cancel           { request_id: u32 },
}

impl<T> Request<T> {
    fn request_id(&self) -> u32 {
        match self {
            Request::ExecuteQuery  { request_id, .. } => *request_id,
            Request::ExecuteStream { request_id, .. } => *request_id,
            Request::Cancel        { request_id }     => *request_id,
        }
    }
}

enum Response<R, E> {
    Row   { request_id: u32, data: R },
    Done  { request_id: u32 },
    Error { request_id: u32, error: E },
}

impl<R, E> Response<R, E> {
    fn request_id(&self) -> u32 {
        match self {
            Response::Row   { request_id, .. } => *request_id,
            Response::Done  { request_id }     => *request_id,
            Response::Error { request_id, .. } => *request_id,
        }
    }

    fn is_terminal(&self) -> bool {
        matches!(self, Response::Done { .. } | Response::Error { .. })
    }
}

// ---------------------------------------------------------------------------
// Raw bidirectional channel transport
// ---------------------------------------------------------------------------

struct Transport<In, Out> {
    tx: mpsc::Sender<Out>,
    rx: mpsc::Receiver<In>,
}

impl<In: Send + 'static, Out: Send + 'static> Transport<In, Out> {
    fn pair(buf: usize) -> (Transport<Out, In>, Transport<In, Out>) {
        let (tx1, rx1) = mpsc::channel(buf);
        let (tx2, rx2) = mpsc::channel(buf);
        (
            Transport { tx: tx1, rx: rx2 }, // client
            Transport { tx: tx2, rx: rx1 }, // server
        )
    }

    async fn send(&self, msg: Out) -> Result<(), TransportClosedError> {
        self.tx.send(msg).await.map_err(|_| TransportClosedError)
    }

    async fn recv(&mut self) -> Option<In> {
        self.rx.recv().await
    }
}

// ---------------------------------------------------------------------------
// Public response types exposed to server handlers
// ---------------------------------------------------------------------------

/// What the server receives for each new request.
pub enum IncomingRequest<T> {
    Query  { query: T, reply: ReplyHandle<(), ()> },  // placeholder — see below
    Stream { query: T },
    Cancel,
}

// ---------------------------------------------------------------------------
// Server reply handles
// ---------------------------------------------------------------------------

/// For single-response queries: consume `self` to send exactly one reply.
pub struct ReplyHandle<R, E> {
    request_id: u32,
    tx: mpsc::Sender<Response<R, E>>,
}

impl<R: Send + 'static, E: Send + 'static> ReplyHandle<R, E> {
    pub async fn respond(self, data: R) -> Result<(), TransportClosedError> {
        self.tx
            .send(Response::Row { request_id: self.request_id, data })
            .await
            .map_err(|_| TransportClosedError)?;
        self.tx
            .send(Response::Done { request_id: self.request_id })
            .await
            .map_err(|_| TransportClosedError)
    }

    pub async fn error(self, error: E) -> Result<(), TransportClosedError> {
        self.tx
            .send(Response::Error { request_id: self.request_id, error })
            .await
            .map_err(|_| TransportClosedError)
    }
}

/// For streaming queries: send any number of rows, then close with `done` or `error`.
pub struct ReplySink<R, E> {
    request_id: u32,
    tx: mpsc::Sender<Response<R, E>>,
}

impl<R: Send + 'static, E: Send + 'static> ReplySink<R, E> {
    pub async fn row(&self, data: R) -> Result<(), TransportClosedError> {
        self.tx
            .send(Response::Row { request_id: self.request_id, data })
            .await
            .map_err(|_| TransportClosedError)
    }

    pub async fn done(self) -> Result<(), TransportClosedError> {
        self.tx
            .send(Response::Done { request_id: self.request_id })
            .await
            .map_err(|_| TransportClosedError)
    }

    pub async fn error(self, error: E) -> Result<(), TransportClosedError> {
        self.tx
            .send(Response::Error { request_id: self.request_id, error })
            .await
            .map_err(|_| TransportClosedError)
    }
}

// ---------------------------------------------------------------------------
// Public response types exposed to the client caller
// ---------------------------------------------------------------------------

/// Yielded by a streaming receiver.
pub enum StreamItem<R, E> {
    Row(R),
    Error(E),
}

/// Returned by `execute_query`.
pub enum QueryResult<R, E> {
    Done,
    Row(R),
    Error(E),
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct MultiplexedClient<T, R, E> {
    wire_tx: mpsc::Sender<Request<T>>,
    response_rx: mpsc::UnboundedReceiver<(u32, Response<R, E>)>,
    subscribers: HashMap<u32, mpsc::UnboundedSender<Response<R, E>>>,
    next_id: u32,
}

impl<T, R, E> MultiplexedClient<T, R, E>
where
    T: Send + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    pub fn new(mut transport: Transport<Response<R, E>, Request<T>>) -> Self {
        let (wire_tx, mut wire_rx) = mpsc::channel::<Request<T>>(256);
        let (demux_tx, response_rx) = mpsc::unbounded_channel::<(u32, Response<R, E>)>();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(req) = wire_rx.recv() => {
                        if transport.send(req).await.is_err() { break; }
                    }
                    Some(resp) = transport.recv() => {
                        let id = resp.request_id();
                        if demux_tx.send((id, resp)).is_err() { break; }
                    }
                    else => break,
                }
            }
        });

        Self { wire_tx, response_rx, subscribers: HashMap::new(), next_id: 1 }
    }

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn route(&mut self, id: u32, resp: Response<R, E>) {
        if let Some(tx) = self.subscribers.get(&id) {
            let terminal = resp.is_terminal();
            let _ = tx.send(resp);
            if terminal { self.subscribers.remove(&id); }
        }
    }

    fn drain(&mut self) {
        while let Ok((id, resp)) = self.response_rx.try_recv() {
            self.route(id, resp);
        }
    }

    /// Send a query and wait for the first response.
    pub async fn execute_query(&mut self, query: T) -> Result<QueryResult<R, E>, TransportClosedError> {
        let id = self.alloc_id();
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.subscribers.insert(id, tx);

        self.wire_tx
            .send(Request::ExecuteQuery { request_id: id, query })
            .await
            .map_err(|_| TransportClosedError)?;

        loop {
            self.drain();
            if let Ok(resp) = rx.try_recv() {
                self.subscribers.remove(&id);
                return Ok(match resp {
                    Response::Row  { data, .. }  => QueryResult::Row(data),
                    Response::Done { .. }        => QueryResult::Done,
                    Response::Error { error, .. } => QueryResult::Error(error),
                });
            }
            let (rid, resp) = self.response_rx.recv().await.ok_or(TransportClosedError)?;
            if rid == id {
                self.subscribers.remove(&id);
                return Ok(match resp {
                    Response::Row  { data, .. }   => QueryResult::Row(data),
                    Response::Done { .. }         => QueryResult::Done,
                    Response::Error { error, .. } => QueryResult::Error(error),
                });
            }
            self.route(rid, resp);
        }
    }

    /// Send a streaming query and return a receiver of `StreamItem`s.
    /// The stream ends when the channel closes (terminal response received).
    pub async fn execute_stream_query(
        &mut self,
        query: T,
    ) -> Result<mpsc::UnboundedReceiver<StreamItem<R, E>>, TransportClosedError> {
        let id = self.alloc_id();

        // Internal raw subscriber
        let (raw_tx, mut raw_rx) = mpsc::unbounded_channel::<Response<R, E>>();
        self.subscribers.insert(id, raw_tx);

        // Public channel — strips request_id and terminal signals
        let (pub_tx, pub_rx) = mpsc::unbounded_channel::<StreamItem<R, E>>();
        tokio::spawn(async move {
            while let Some(resp) = raw_rx.recv().await {
                match resp {
                    Response::Row   { data, .. }  => { let _ = pub_tx.send(StreamItem::Row(data)); }
                    Response::Error { error, .. } => { let _ = pub_tx.send(StreamItem::Error(error)); break; }
                    Response::Done  { .. }        => break,
                }
            }
        });

        self.wire_tx
            .send(Request::ExecuteStream { request_id: id, query })
            .await
            .map_err(|_| TransportClosedError)?;

        Ok(pub_rx)
    }

    /// Cancel an in-flight request by the receiver returned from `execute_stream_query`.
    /// Pass the `request_id`-free cancel token obtained from `stream_cancel_token()`.
    pub async fn cancel(&mut self, id: CancelToken) -> Result<(), TransportClosedError> {
        self.subscribers.remove(&id.0);
        self.wire_tx
            .send(Request::Cancel { request_id: id.0 })
            .await
            .map_err(|_| TransportClosedError)
    }

    /// Drive pending inbound messages into subscriber queues.
    /// Call this in your main `select!` loop.
    pub fn drive(&mut self) {
        self.drain();
    }
}

/// An opaque token for cancelling a stream. Hides the request_id from callers.
pub struct CancelToken(u32);

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

/// What the server sees for each incoming request — no request_id visible.
pub enum ServerRequest<T, R, E> {
    Query  { query: T, reply: ReplyHandle<R, E> },
    Stream { query: T, sink: ReplySink<R, E> },
    Cancel,
}

pub struct MultiplexedServer<T, R, E> {
    incoming_rx: mpsc::UnboundedReceiver<ServerRequest<T, R, E>>,
}

impl<T, R, E> MultiplexedServer<T, R, E>
where
    T: Send + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    pub fn new(mut transport: Transport<Request<T>, Response<R, E>>) -> Self {
        let (wire_tx, mut wire_rx) = mpsc::channel::<Response<R, E>>(256);
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<ServerRequest<T, R, E>>();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(req) = transport.recv() => {
                        let id = req.request_id();
                        let handle = ReplyHandle { request_id: id, tx: wire_tx.clone() };
                        let server_req = match req {
                            Request::ExecuteQuery  { query, .. } => ServerRequest::Query  { query, reply: ReplyHandle { request_id: id, tx: wire_tx.clone() } },
                            Request::ExecuteStream { query, .. } => ServerRequest::Stream { query, sink:  ReplySink  { request_id: id, tx: wire_tx.clone() } },
                            Request::Cancel        { .. }        => ServerRequest::Cancel,
                        };
                        if incoming_tx.send(server_req).is_err() { break; }
                    }
                    Some(resp) = wire_rx.recv() => {
                        if transport.send(resp).await.is_err() { break; }
                    }
                    else => break,
                }
            }
        });

        Self { incoming_rx }
    }

    /// Returns the next request, or `None` if the client disconnected.
    pub async fn next_request(&mut self) -> Option<ServerRequest<T, R, E>> {
        self.incoming_rx.recv().await
    }
}

// // ---------------------------------------------------------------------------
// // Example
// // ---------------------------------------------------------------------------

// #[derive(Debug)] pub struct MyQuery  { pub sql: String }
// #[derive(Debug)] pub struct MyRow    { pub value: String }
// #[derive(Debug)] pub struct MyError  { pub message: String }

// #[tokio::main]
// async fn main() {
//     let (client_transport, server_transport) =
//         Transport::<Response<MyRow, MyError>, Request<MyQuery>>::pair(256);

//     tokio::spawn(async move {
//         let mut server = MultiplexedServer::<MyQuery, MyRow, MyError>::new(server_transport);
//         while let Some(req) = server.next_request().await {
//             tokio::spawn(async move {
//                 match req {
//                     ServerRequest::Query { query, reply } => {
//                         println!("[server] single query: {}", query.sql);
//                         reply.done().await.unwrap();
//                     }
//                     ServerRequest::Stream { query, sink } => {
//                         println!("[server] stream query: {}", query.sql);
//                         for i in 0..3 {
//                             sink.row(MyRow { value: format!("row {i}") }).await.unwrap();
//                         }
//                         sink.done().await.unwrap();
//                     }
//                     ServerRequest::Cancel => println!("[server] cancel"),
//                 }
//             });
//         }
//     });

//     let mut client = MultiplexedClient::<MyQuery, MyRow, MyError>::new(client_transport);

//     // Single query
//     let result = client.execute_query(MyQuery { sql: "SELECT 1".into() }).await.unwrap();
//     println!("[client] query result: {result:?}");

//     // Streaming
//     let mut stream = client
//         .execute_stream_query(MyQuery { sql: "SELECT * FROM logs".into() })
//         .await
//         .unwrap();

//     while let Some(item) = stream.recv().await {
//         match item {
//             StreamItem::Row(row)     => println!("[client] row: {}", row.value),
//             StreamItem::Error(e)     => println!("[client] error: {}", e.message),
//         }
//     }
// }