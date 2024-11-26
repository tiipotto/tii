//! Humpty WebSocket is a crate which extends Humpty Core with WebSocket support by hooking into the latter's `WebsocketHandler` trait. It handles the WebSocket handshake and framing protocol and provides a simple and flexible API for sending and receiving messages. Using Humpty's generic `Stream` type, it supports drop-in TLS. It also has no dependencies in accordance with Humpty's goals of being dependency-free.
//!
//! It provides both synchronous and asynchronous WebSocket functionality.

#![warn(missing_docs)]



pub mod message;
pub mod stream;

mod frame;
