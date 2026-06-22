# Guide: Talking to a Chatbot over HTTP

A Pyroduct chatbot is a session-based HTTP service. You send messages, it
replies, and it remembers the conversation history on the server side. You only
ever send the **current message** — the server tracks the rest.

Each conversation is identified by a numeric **session ID**.

---

## Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/` | Start a **new conversation**. Returns a `session_id` in the response. |
| `POST` | `/{session_id}` | Send the next message in an **existing conversation**. |
| `GET`  | `/schema` | Inspect the expected input/output fields. |

---

## curl Examples

Assume the chatbot is running on `http://naked-chatbot:8080`.

### Check the schema (optional)

```bash
curl http://naked-chatbot:8080/schema
```

### Start a conversation

```bash
curl -X POST http://naked-chatbot:8080/ \
  -H 'Content-Type: application/json' \
  -d '{
    "role": "user",
    "content": "Hello, who are you?"
  }'
```

Response:

```json
{
  "role": "assistant",
  "content": "Hi! I am a helpful assistant. How can I help you today?",
  "session_id": 1
}
```

> [!IMPORTANT]
> Save the `session_id` — you need it for every follow-up message.

### Continue the conversation

Use the `session_id` in the URL path:

```bash
curl -X POST http://naked-chatbot:8080/1 \
  -H 'Content-Type: application/json' \
  -d '{
    "role": "user",
    "content": "Tell me a joke about programming"
  }'
```

You can keep posting to `/{session_id}` for as many turns as you like.

### Session endings

The chatbot controls when a conversation ends:

| Behavior | What you get back |
|----------|-------------------|
| Conversation continues | Normal JSON response with the reply. |
| Conversation ends gracefully | One last JSON response, then the session is closed. |
| Conversation terminated | Empty `{}` response. The session is closed. |

After a session ends, that `session_id` is no longer usable.

---

## Python Example

```python
import requests

BASE_URL = "http://naked-chatbot:8080"

def start_conversation(message: str) -> tuple[int, dict]:
    """Send the first message. Returns (session_id, response)."""
    resp = requests.post(
        f"{BASE_URL}/",
        json={"role": "user", "content": message},
    )
    resp.raise_for_status()
    data = resp.json()
    return data.pop("session_id"), data

def send_message(session_id: int, message: str) -> dict:
    """Send a follow-up message in an existing session."""
    resp = requests.post(
        f"{BASE_URL}/{session_id}",
        json={"role": "user", "content": message},
    )
    resp.raise_for_status()
    data = resp.json()
    data.pop("session_id", None)
    return data

# --- Usage ---
session_id, reply = start_conversation("Hi there!")
print(f"Session {session_id}: {reply['content']}")

reply = send_message(session_id, "What can you do?")
print(f"Bot: {reply['content']}")

reply = send_message(session_id, "Thanks, goodbye!")
print(f"Bot: {reply['content']}")
```

---

## Quick Reference

**Start a new session:**
```bash
curl -s -X POST http://HOST:PORT/ \
  -H 'Content-Type: application/json' \
  -d '{"role": "user", "content": "YOUR MESSAGE"}'
```

**Continue a session:**
```bash
curl -s -X POST http://HOST:PORT/SESSION_ID \
  -H 'Content-Type: application/json' \
  -d '{"role": "user", "content": "YOUR MESSAGE"}'
```

**Check the schema:**
```bash
curl -s http://HOST:PORT/schema | python3 -m json.tool
```

---

## Common Mistakes

| Problem | Fix |
|---------|-----|
| Forgot `Content-Type` header | Add `-H 'Content-Type: application/json'` (or set `json=` in `requests`). |
| Sending conversation history yourself | Don't — the server manages it. Only send the current message. |
| Wrong field names | Check `/schema`. For a chat module it's `{"role": "...", "content": "..."}`. |
| Using a string session ID in the URL | The `session_id` is a **number**, not a string UUID. |
| Posting to `/` every time | That creates a **new** session each time. Use `/{session_id}` to continue. |
