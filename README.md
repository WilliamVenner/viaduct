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
pub enum ExampleResponse {
    FrontflipOk,
    BackflipOk,
}
```

## Parent process

```rs
let child = std::process::Command::new("child.exe");
let ((tx, rx), mut child) = viaduct::ViaductBuilder::parent(child).unwrap();

std::thread::spawn(move || {
    rx.run(
        |rpc: ExampleRpc| match rpc {
            ExampleRpc::Cow => println!("Moo"),
            ExampleRpc::Pig => println!("Oink"),
            ExampleRpc::Horse => println!("Neigh"),
        },

        |request: ExampleRequest| match request {
            ExampleRequest::DoAFrontflip => {
                println!("Doing a frontflip!");
                ExampleResponse::FrontflipOk
            },

            ExampleRequest::DoABackflip => {
                println!("Doing a backflip!");
                ExampleResponse::BackflipOk
            },
        },
    ).unwrap();
});

tx.rpc(ExampleRpc::Cow).unwrap();
tx.rpc(ExampleRpc::Pig).unwrap();
tx.rpc(ExampleRpc::Horse).unwrap();

let response = tx.request(ExampleRequest::DoAFrontflip).unwrap();
assert_eq!(response, ExampleResponse::FrontflipOk);
```

## Child process

```rs
let (tx, rx) = viaduct::ViaductBuilder::child().unwrap();

std::thread::spawn(move || {
    rx.run(
        |rpc: ExampleRpc| match rpc {
            ExampleRpc::Cow => println!("Moo"),
            ExampleRpc::Pig => println!("Oink"),
            ExampleRpc::Horse => println!("Neigh"),
        },

        |request: ExampleRequest| match request {
            ExampleRequest::DoAFrontflip => {
                println!("Doing a frontflip!");
                ExampleResponse::FrontflipOk
            },

            ExampleRequest::DoABackflip => {
                println!("Doing a backflip!");
                ExampleResponse::BackflipOk
            },
        },
    ).unwrap();
});

tx.rpc(ExampleRpc::Horse).unwrap();
tx.rpc(ExampleRpc::Pig).unwrap();
tx.rpc(ExampleRpc::Cow).unwrap();

let response = tx.request(ExampleRequest::DoABackflip).unwrap();
assert_eq!(response, ExampleResponse::BackflipOk);
```

# Use Cases

Viaduct was designed for separating user interface from application logic in a cross-platform manner.

For example, an application may want to run a GUI in a separate process from the application logic, for modularity or performance reasons.

Viaduct allows for applications like this to communicate between these processes in a natural way, without having to manually implement IPC machinery & synchronization.

# Usage

## Serialization

Viaduct currently supports serialization and deserialization of data using [`bincode`](https://docs.rs/bincode) or [`speedy`](https://docs.rs/speedy) at your choice, using the respective Cargo feature flags.

You can also manually implement the [`Pipeable`] trait.

## Initializing a viaduct

A viaduct is initialized by calling [`ViaductBuilder::parent`] as the parent process, which will spawn your child process.

Your child process should then call [`ViaductBuilder::child`], [`ViaductBuilder::child_with_args_os`], [`ViaductBuilder::child_with_args`] (see CAVEAT below) to bridge the connection between the parent and child.

Then, you are ready to start...

## Passing data

Viaduct has two modes of operation: RPCs and Requests/Responses.

RPCs are one-way messages, and are useful for sending notifications to the other process.

Requests/Responses are two-way messages, and are useful for sending requests to the other process and receiving data as a response.

Requests will block any other thread trying to send requests and RPCs through the viaduct, until a response is received.

## CAVEAT: Don't use [`std::env::args_os`] or [`std::env::args`] in your child process!

The child process should not use `args_os` or `args` to get its arguments, as these will contain data Viaduct needs to pass to the child process.

Instead, use the argument iterator provided by [`ViaductBuilder::child_with_args_os`] or [`ViaductBuilder::child_with_args`] for `args_os` and `args` respectively.

# License

Viaduct is licensed under the MIT license or the Apache License v2.0, at your choice.