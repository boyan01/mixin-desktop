# mixin-headless

`mixin-headless` runs the Mixin desktop account runtime without a UI, so a
dedicated Mixin account can stay online on a machine and be used by local
agents and harnesses. It serves two things on `127.0.0.1`:

- `/mcp`: the Mixin MCP server (Streamable HTTP, stateless, JSON responses).
- `/events`: server-sent events for incoming messages.

It does not run agents, keep agent sessions, or decide when to reply. That
belongs to the harness that consumes these endpoints.

## Quick start

Use a dedicated Mixin account. Linking the same account to another desktop
client signs this one out.

```sh
cargo build -p mixin_desktop_headless --release
export MIXIN_DESKTOP_DATA_DIR=~/mixin-headless-data

# Scan the QR code with the Mixin app of the dedicated account.
# Expired codes are replaced every 60 seconds.
./target/release/mixin-headless login

# Keep the account online. Add --allow-send to enable the send tools.
./target/release/mixin-headless serve --port 55002 --allow-send

# Print the bearer token for /mcp and /events.
./target/release/mixin-headless token
```

`MIXIN_DESKTOP_DATA_DIR` is required. Without it the runtime would fall back to
the desktop app's data directory and share its account and database. Logs are
written to `$MIXIN_DESKTOP_DATA_DIR/log`.

The default port is 55001, which is also the desktop app's MCP port. Pass
`--port` when both run on the same machine.

## MCP

Every request needs `Authorization: Bearer <token>`.

```sh
curl -s http://127.0.0.1:55002/mcp \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Accept: application/json, text/event-stream' \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

Tool names, descriptions, and input schemas are served by `tools/list`; they
are not repeated here. `mixin_get_app_status` reports the enabled permission
scopes and tools.

Read tools are always available. The send tools (`mixin_send_text`,
`mixin_send_image`, `mixin_send_sticker`, `mixin_send_contact`,
`mixin_send_app_card`) exist only when `serve` runs with `--allow-send`. The
desktop apps cannot enable them.

## Events

`GET /events` streams messages from other users that an agent should see.

```sh
curl -N "http://127.0.0.1:55002/events?cursor=123" \
  -H "Authorization: Bearer $TOKEN"
```

### Cursor

Each message event's SSE `id` is a cursor. The harness stores the last cursor
it handled and resumes with `?cursor=<cursor>` or a `Last-Event-ID` header;
events after that cursor are replayed first, then new ones follow. Without a
cursor, the stream starts at the newest message and only delivers new ones.
The server keeps no delivery state.

An invalid cursor returns `400`. A missing or wrong token returns `401`.

### Event types

`ready` is sent once after connecting, with the cursor the stream starts from:

```
event: ready
data: {"cursor":"123"}
```

`message` is sent for each triggering message:

```
event: message
id: 124
data: {"cursor":"124","trigger":"mention","conversation":{...},"message":{...}}
```

```json
{
  "cursor": "124",
  "trigger": "mention",
  "conversation": {
    "conversation_id": "<conversation-id>",
    "category": "GROUP",
    "name": "Dev"
  },
  "message": {
    "message_id": "<message-id>",
    "conversation_id": "<conversation-id>",
    "sender_id": "<sender-user-id>",
    "sender_name": "Alice",
    "sender_identity_number": "7000",
    "category": "SIGNAL_TEXT",
    "content": "@7001 can you check this?",
    "quote_message_id": null,
    "sticker_id": null,
    "shared_user_id": null
  }
}
```

`message` has the same fields as `mixin_get_message` (abbreviated above).
Mentions stay in raw `@<identity_number>` form.

`error` is sent before the stream closes when reading messages fails:

```
event: error
data: {"message":"..."}
```

The stream also closes when the account runtime or server stops. Reconnect with
the last cursor.

### Triggers

| Message | `trigger` |
|---|---|
| Direct conversation that is not muted | `direct` |
| Mentions this account (text messages) | `mention` |
| Quotes a message from this account | `quote` |
| Any other message in a group that is not muted | `group` |
| Muted conversation without a mention or quote | not sent |
| Sent by this account, or a recall | not sent |

Group system messages, such as this account being added to a group, arrive as
`group` events with a `SYSTEM_*` category. Mute a group to receive only
mentions and quotes from it.

## Running under launchd

`serve` exits on `SIGTERM` after shutting the account down, and exits with an
error when the account runtime stops, so `KeepAlive` restarts it. Run `login`
once by hand first.

`~/Library/LaunchAgents/one.mixin.headless.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>one.mixin.headless</string>
  <key>ProgramArguments</key>
  <array>
    <string>/Users/me/bin/mixin-headless</string>
    <string>serve</string>
    <string>--port</string>
    <string>55002</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>MIXIN_DESKTOP_DATA_DIR</key>
    <string>/Users/me/mixin-headless-data</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>ThrottleInterval</key>
  <integer>30</integer>
  <key>StandardErrorPath</key>
  <string>/Users/me/mixin-headless-data/serve.log</string>
</dict>
</plist>
```

```sh
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/one.mixin.headless.plist
launchctl kickstart -k gui/$(id -u)/one.mixin.headless
```

Paths in the plist must be absolute. Add `--allow-send` to
`ProgramArguments` only when the harness should send messages.

## Security

- Both endpoints listen on `127.0.0.1` only. For a harness on another machine,
  use an SSH tunnel, for example `ssh -N -L 55002:127.0.0.1:55002 mac-mini`.
- Treat the bearer token like the account: it grants read access to every
  conversation, and send access with `--allow-send`.
- Message content is untrusted input. Anyone in a group can write text that
  tries to steer an agent. Give agents that read these messages only the tools
  they need, keep `--allow-send` off unless replies are required, and let the
  harness decide which conversations an agent may reply to.

## Limitations

- If the session is revoked while `serve` is running (for example, the device
  is removed from the phone), `serve` does not exit until the runtime stops.
  The next start fails with "no signed-in account"; run `login` again.
- Images are sent without a thumbnail, so recipients see a placeholder until
  the image downloads.
- App cards must carry a real bot `app_id`, and `actions` require `icon_url`.
