# WebSocket Chat Server

A real-time chat server built using Rust with support for group and private messaging through WebSocket connections. This server allows users to join and leave chat rooms, send group messages to a room, or have private conversations with other users.

### **note:**The project is not complete and need to develop,it will be build as fast as I can.

## Features

- **Real-time Communication**: Supports WebSocket-based messaging for low-latency, bi-directional communication.
- **Group Chat**: Users can join rooms and broadcast messages to all participants in the room.
- **Private Chat**: Users can send direct messages to others.
- **Token-based Authentication**: Optional token-based authentication for user sessions.
- **Scalable Architecture**: The server is designed to handle multiple concurrent connections.

## Table of Contents

- [Installation](#installation)
- Usage
  - [Starting the Server](#starting-the-server)
  - [Client Interaction](#client-interaction)
- [Configuration](#configuration)
- Message Format
  - [Enter Room](#enter-room)
  - [Leave Room](#leave-room)
  - [Group Chat](#group-chat)
  - [Private Chat](#private-chat)
- [Error Handling](#error-handling)
- [Contributing](#contributing)
- [License](#license)

## Installation

To get started with the `websocket_chat_server`, follow the instructions below.

### Prerequisites

Ensure you have the following installed on your system:

- Rust (latest stable version)
- Cargo

### Clone the Repository

```
git clone https://github.com/yourusername/websocket_chat_server.git
cd websocket_chat_server
```

### Build the Project

You can build the project with Cargo:

```
cargo build --bin testserver --release
```

### Run the Project

After building the project, you can run the server with:

```
cargo run --bin testserver --release
```

## Usage

### Starting the Server

To start the WebSocket chat server, run the following command:

```
cargo run --bin testserver --release
```

The server will listen for WebSocket connections on `ws://127.0.0.1:8080/`. You can adjust the server settings in the configuration section below.

### Client Interaction

Clients can connect to the server using WebSocket clients such as a browser, WebSocket libraries, or custom clients. The server supports the following message types for interaction:

- **Enter Room**: Join a chat room.
- **Leave Room**: Leave the current room.
- **Group Chat**: Send a message to all users in the room.
- **Private Chat**: Send a direct message to a specific user.

## Configuration

The server can be configured by modifying the environment variables or the configuration file. Key settings include:

- **Port**: The port the server listens on (default: `8080`).
- **Max Connections**: Maximum number of WebSocket connections allowed.
- **Authentication**: Enable or disable token-based authentication.

You can modify these settings in the `.env` file or pass them as command-line arguments.

## Message Format

Each WebSocket message consists of a `message_type` and a `message_body`. Below is the format for different message types.

### Enter Room

```json
{
  "message_type": "Enter",
  "message_body": {
    "sender_username": "your_username"
  },
  "token": "optional_uuid_token"
}
```

### Leave Room

```json
{
  "message_type": "Leave",
  "message_body": {
    "sender_username": "your_username"
  },
  "token": "optional_uuid_token"
}
```

### Group Chat

```json
{
  "message_type": "GroupChat",
  "message_body": {
    "sender_username": "your_username",
    "content": "your_message"
  },
  "token": "optional_uuid_token"
}
```

### Private Chat

```json
{
  "message_type": "PrivateChat",
  "message_body": {
    "target_username": "target_user",
    "sender_username": "your_username",
    "content": "your_message"
  },
  "token": "optional_uuid_token"
}
```

## Error Handling

The server provides basic error handling and logging. If a WebSocket connection is closed or an invalid message is received, the server will log the error and terminate the connection if necessary.

Common error scenarios include:

- **Invalid Message Format**: The server will log an error if a malformed message is received.
- **Authentication Errors**: If a message is sent without a valid token (in token-based mode), the server will reject the message.

## Contributing

Contributions are welcome! Please follow the steps below to contribute to the project:

1. Fork the repository.

2. Create a new branch for your feature or bug fix:

   ```
   git checkout -b my-feature-branch
   ```

3. Make your changes and commit them:

   ```
   git commit -m "Add some feature"
   ```

4. Push the branch:

   ```
   git push origin my-feature-branch
   ```

5. Open a Pull Request.

### Code Style

Please ensure your code follows Rust’s standard code style. You can use `cargo fmt` to automatically format your code.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
