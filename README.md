# HTTP LLM Proxy Stream

HTTP proxy server built with Rust, featuring streaming support and request/response logging.

## Features

- **HTTP Request Proxying** - redirect all requests to a specified target URL
- **Streaming Support** - real-time data transmission
- **Logging** - record all requests and responses to a CSV file
- **Gzip Support** - automatic decoding of compressed responses
- **Customizable Headers** - ability to add additional HTTP headers
- **CLI Configuration** - flexible setup via command-line arguments

## Installation

### Requirements

- Rust 2024 edition or newer
- Cargo

### Build

```bash
cargo build --release
```

## Usage

### Basic Usage

```bash
cargo run -- --target-url https://api.openai.com/v1
```

### Full Example

```bash
cargo run -- \
  --port 8080 \
  --target-url https://api.example.com \
  --add-header "Authorization: Bearer YOUR_API_KEY" \
  --log-file requests.log
```

### Command-Line Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--port` | `-p` | Port to listen on | `8080` |
| `--target-url` | `-t` | URL to proxy requests to | **required** |
| `--add-header` | `-a` | Additional header in format "Name: Value" | - |
| `--log-file` | `-l` | Log file name | `log.csv` |
| `--timeout` | `-T` | Request timeout in minutes | `3` |

## Examples

### Proxying to OpenAI API

```bash
cargo run -- \
  --target-url https://api.openai.com/v1 \
  --add-header "Authorization: Bearer sk-..." \
  --port 8080
```

Now you can send requests:

```bash
curl -X POST http://localhost:8080/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### Using with Another API

```bash
cargo run -- \
  --target-url https://api.anthropic.com/v1 \
  --add-header "x-api-key: YOUR_KEY"
```

## Log Format

Logs are written in CSV format with the following columns:

1. **Method** - HTTP method (GET, POST, PUT, DELETE, PATCH)
2. **URL** - full request URL
3. **Request Body** - request body in JSON format
4. **Response Body** - response body (with automatic gzip decoding)

Example log entry:

```csv
POST,http://localhost:8080/chat/completions,"{""model"": ""gpt-3.5-turbo""}","{""choices"": [{""message"": {""content"": ""Hello!""}}]}"
```

## Dependencies

- `actix-web` - web framework
- `reqwest` - HTTP client with streaming support
- `tokio` - async runtime
- `serde` / `serde_json` - serialization/deserialization
- `flate2` - gzip decoding
- `bytes` - byte data manipulation
- `futures-util` - futures/streams utilities
- `log` / `env_logger` - logging
- `clap` - CLI argument parsing

## License

MIT
