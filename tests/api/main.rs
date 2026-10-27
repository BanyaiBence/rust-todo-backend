//! HTTP-level tests driving the full router in-process against PostgreSQL.

#[path = "../common/mod.rs"]
mod common;

mod auth;
mod cors;
mod health;
mod todos;
