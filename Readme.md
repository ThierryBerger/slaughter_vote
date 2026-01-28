# Theme Voting System

A voting system, built with Bevy Game Jam in mind.

Users vote yes/no/skip on theme suggestions through a CLI client.

## Stack

- Rust (Axum+sqlx backend, CLI client)
- Supabase (PostgreSQL database + Discord OAuth)
- Discord API (manual import has to be done)
- TODO: deploy (probably fly.io)

## Project structure

- crates/client : CLI voting application
- crates/server : Backend API server + theme loader

## Authentication

The client starts a local HTTP server on port 8080 to handle Discord OAuth callbacks from Supabase. After authentication, it fetches themes from the backend and submits votes.
