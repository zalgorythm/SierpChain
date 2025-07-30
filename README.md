# SierpChain

SierpChain is a novel blockchain system that uses Sierpinski triangle fractal patterns as a core component of its mining, cryptography, and consensus mechanisms. This project is an exploration of how complex geometric patterns can be integrated into blockchain technology.

## Features

- **Fractal-Based Proof-of-Work**: A unique mining algorithm where miners generate Sierpinski triangles of a certain complexity.
- **Visual Block Representation**: Each block in the chain is associated with a unique fractal triangle, providing a visual representation of the blockchain's state.
- **Rust-Powered Backend**: The core of SierpChain is built with Rust, ensuring performance and safety. It uses `actix-web` to provide a REST API.
- **Web-Based Frontend**: A responsive frontend built with Yew, a modern Rust framework for building multi-threaded frontend web apps with WebAssembly.
- **Real-Time Updates**: The frontend fetches and displays new blocks as they are mined and added to the chain.

## Getting Started

These instructions will get you a copy of the project up and running on your local machine for development and testing purposes.

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

### Building and Running

1.  **Clone the repository:**

    ```bash
    git clone https://github.com/zalgorythm/sierpchain.git
    cd sierpchain
    ```

2.  **Run the backend:**

    Open a terminal and run the following commands:

    ```bash
    cargo build
    cargo run
    ```

    The backend server will start on `http://127.0.0.1:8080`.

3.  **Run the frontend:**

    Open another terminal and navigate to the `frontend` directory:

    ```bash
    cd frontend
    ```

    You'll need a simple HTTP server to serve the frontend files. If you don't have one, you can use `basic-http-server`:

    ```bash
    cargo install basic-http-server
    basic-http-server .
    ```

    Or use python's built-in http server:
    ```bash
    python3 -m http.server
    ```

    Open your web browser and navigate to `http://127.0.0.1:4000` (or the port specified by your http server).

## Project Structure

-   `src/`: Contains the core backend modules.
    -   `main.rs`: The entry point for the backend application. It sets up the `actix-web` server and the mining loop.
    -   `block.rs`: Defines the `Block` and `Blockchain` structs and the core blockchain logic.
    -   `fractal.rs`: Defines the `FractalTriangle` struct and the logic for generating Sierpinski triangles.
-   `frontend/`: Contains the web UI client.
    -   `src/main.rs`: The entry point for the Yew frontend application.
    -   `index.html`: The main HTML file for the frontend.
-   `Cargo.toml`: The manifest file for the Rust backend project.
-   `README.md`: This file.

## API Endpoints

### `GET /blocks`

Retrieves the entire blockchain.

-   **Method**: `GET`
-   **URL**: `/blocks`
-   **Success Response**:
    -   **Code**: `200 OK`
    -   **Content**: A JSON array of `Block` objects.

    **Example:**

    ```json
    [
      {
        "index": 0,
        "timestamp": 1678886400,
        "fractal": {
          "depth": 0,
          "vertices": [
            [0.0, 0.0],
            [1.0, 0.0],
            [0.5, 0.866]
          ]
        },
        "data": "Genesis Block",
        "previous_hash": "0",
        "hash": "...",
        "nonce": 0
      }
    ]
    ```

## Contributing

Contributions are welcome! Please see our [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to contribute.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
