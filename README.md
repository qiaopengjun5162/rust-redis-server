# Rust Redis Server

Rust Redis Server is a simple Redis server implemented in Rust.
It is a work in progress and is not yet feature complete.

## Features

- Simple command handling
- Basic data types (strings, lists, sets, hashes)

## Installation

To install the server, clone the repository and run:

```bash
cargo build --release
```

## Usage

To run the server, use the following command:

```bash
cargo run --release
```

The server will start on port 6379 by default. You can change the port by setting the `REDIS_PORT` environment variable.

## Contributing

Contributions are welcome! Please submit pull requests with any changes you make.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
