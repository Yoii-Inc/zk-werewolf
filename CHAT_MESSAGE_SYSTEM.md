# Chat Message System Implementation

## Overview
Implemented a system that displays messages generated on the server side and client side together, and can be restored when reloaded.

## Architecture

### Message Classification
1. **Server-side messages** (`source: "server"`)
   - Chat messages sent from the server
   - Received in real-time via WebSocket
   - Periodically fetched via polling API
   - Example: User chat, system notifications

2. **Client-side messages** (`source: "client"`)
   - Messages generated on the frontend
   - Divination results, error messages, etc.
   - Persisted to localStorage
   - Example: "Divination result: XXX is a werewolf", "Error: XXX"

### Data Flow

```
Server-side messages:
  ┌─────────────┐
  │   Server    │
  └──────┬──────┘
         │
    ┌────┴─────┐
    │          │
 WebSocket   Polling
    │          │
    └────┬─────┘
         ▼
  addServerMessage()
         │
         ▼
  serverMessages (State)


Client-side messages:
  ┌──────────────┐
  │ Client Logic │
  └──────┬───────┘
         │
    addMessage()
         │
         ▼
  clientMessages (State)
         │
         ▼
   localStorage
```

Merge:
```
serverMessages + clientMessages
         │
    (Sort by timestamp)
         │
         ▼
  Display messages
```

## Implementation Details

### 1. ChatMessage Type Extension
```typescript
export interface ChatMessage {
  id: string;
  sender: string;
  message: string;
  timestamp: string;
  type: "system" | "normal" | "whisper";
  source?: "server" | "client"; // Added (optional)
}
```

### 2. useGameChat Hook Features
- **serverMessages**: Holds server-side messages (memory only)
- **clientMessages**: Holds client-side messages (persisted to localStorage)
- **messages**: Merges both and sorts by timestamp

#### Main Functions
- `addMessage(message)`: Add client-side message
  - Automatically sets `source` to `"client"` if not set
  - Saves to localStorage
  
- `addServerMessage(message)`: Add server-side message
  - Used when receiving messages in real-time via WebSocket
  - `source` is always `"server"`

- `setMessages(apiMessages)`: Set array of messages from server
  - Used for batch updates via polling
  - Attaches `source: "server"` to each message

- `resetMessages()`: Clear all messages
  - Used when game resets
  - Also clears localStorage

### 3. Restoration on Reload
1. Upon useGameChat initialization, restore `clientMessages` from localStorage
2. useGameInfo retrieves `serverMessages` from server
3. Both are automatically merged and displayed

### 4. Duplicate Prevention
- Server-side messages are unique by ID and timestamp
- Client-side messages are generated with unique ID using `Date.now()`
- Possibility of receiving same message via WebSocket and polling, but managed on server side

## Usage Examples

### Adding Client-side Messages
```typescript
// Display divination result
addMessage({
  id: Date.now().toString(),
  sender: "System",
  message: `Divination result: ${playerName} is a ${role}`,
  timestamp: new Date().toISOString(),
  type: "system",
  // source is automatically set to "client"
});
```

### Adding Server-side Messages (WebSocket)
```typescript
// Automatically processed inside useGameWebSocket
addServerMessage({
  id: "Server",
  sender: fullMessage.player_name,
  message: fullMessage.content,
  timestamp: new Date().toISOString(),
  type: "normal",
  source: "server",
});
```

## Testing Methods

### 1. Basic Operation Test
1. Join a game and send a chat message
2. Verify divination result as a Seer
3. Confirm both messages display in chronological order

### 2. Reload Test
1. Reload the page during gameplay
2. Confirm server-side messages (chat) are restored
3. Confirm client-side messages (divination results, etc.) are also restored

### 3. Game Reset Test
1. Reset the game
2. Confirm all messages are cleared
3. Confirm localStorage is also cleared

## Future Improvements

### 1. Complete Elimination of Duplicate Messages
Currently, there's a possibility of receiving the same message via WebSocket and polling.
Can implement duplicate checking using server-side message IDs.

### 2. Message History Limitations
Consider localStorage capacity limitations and automatically delete old messages.

### 3. Message Filtering
- Per-user hide settings
- Filter by message type

### 4. Performance Optimization
- Virtual scrolling for cases with large number of messages
- Message pagination
