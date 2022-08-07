[![crates.io](https://img.shields.io/crates/v/viaduct.svg)](https://crates.io/crates/viaduct)
[![docs.rs](https://docs.rs/viaduct/badge.svg)](https://docs.rs/viaduct/)
[![license](https://img.shields.io/crates/l/viaduct)](https://github.com/WilliamVenner/viaduct/blob/master/LICENSE)

Viaduct is a library for establishing a duplex communication channel between a parent and child process, using unnamed pipes.

# Example

## Shared library

```rs
#[derive(Serialize, Deserialize)]
pub enum ExampleRpc {
    Cow,
    Pig,
    Horse
}

#[derive(Serialize, Deserialize)]
pub enum ExampleRequest {
    DoAFrontflip,
    DoABackflip,
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct FrontflipError;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct BackflipError;
```

## Parent process

```rust
let child = std::process::Command::new("child.exe");
let ((tx, rx), mut child) = viaduct::ViaductBuilder::parent(child).unwrap().build().unwrap();

std::thread::spawn(move || {
    rx.run(
        |rpc: ExampleRpc| match rpc {
            ExampleRpc::Cow => println!("Moo"),
            ExampleRpc::Pig => println!("Oink"),
            ExampleRpc::Horse => println!("Neigh"),
        },

        |request: ExampleRequest, tx| match request {
            ExampleRequest::DoAFrontflip => {
                println!("Doing a frontflip!");
                tx.respond(Ok::<_, FrontflipError>(()))
            },

            ExampleRequest::DoABackflip => {
                println!("Doing a backflip!");
                tx.respond(Ok::<_, BackflipError>(()))
            },
        },
    ).unwrap();
});

tx.rpc(ExampleRpc::Cow).unwrap();
tx.rpc(ExampleRpc::Pig).unwrap();
tx.rpc(ExampleRpc::Horse).unwrap();

let response: Result<(), FrontflipError> = tx.request(ExampleRequest::DoAFrontflip).unwrap();
assert_eq!(response, Ok(()));
```

## Child process

```rust
let (tx, rx) = unsafe { viaduct::ViaductBuilder::child() }.unwrap();

std::thread::spawn(move || {
    rx.run(
        |rpc: ExampleRpc| match rpc {
            ExampleRpc::Cow => println!("Moo"),
            ExampleRpc::Pig => println!("Oink"),
            ExampleRpc::Horse => println!("Neigh"),
        },

        |request: ExampleRequest, tx| match request {
            ExampleRequest::DoAFrontflip => {
                println!("Doing a frontflip!");
                tx.respond(Ok::<_, FrontflipError>(()))
            },

            ExampleRequest::DoABackflip => {
                println!("Doing a backflip!");
                tx.respond(Ok::<_, BackflipError>(()))
            },
        },
    ).unwrap();
});

tx.rpc(ExampleRpc::Horse).unwrap();
tx.rpc(ExampleRpc::Pig).unwrap();
tx.rpc(ExampleRpc::Cow).unwrap();

let response: Result<(), BackflipError> = tx.request(ExampleRequest::DoABackflip).unwrap();
assert_eq!(response, Ok(()));
```

# Use Cases

Viaduct was designed for separating user interface from application logic in a cross-platform manner.

For example, an application may want to run a GUI in a separate process from the application logic, for modularity or performance reasons.

Viaduct allows for applications like this to communicate between these processes in a natural way, without having to manually implement IPC machinery & synchronization.

# Usage

Check out [the documentation](https://docs.rs/viaduct/) for usage information.

# License

Viaduct is licensed under the MIT license or the Apache License v2.0, at your choice.