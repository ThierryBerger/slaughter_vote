# Theme Voting System

A voting system, built with Bevy Game Jam in mind.

Users vote yes/no/skip on theme suggestions through a CLI client.

## Stack

- Rust (Axum+sqlx backend, CLI client)
- Supabase (PostgreSQL database + Discord OAuth)
- Discord API (manual import of themes has to be done)
- TODO: deploy (probably fly.io)

## Project structure

- crates/client : CLI voting application
- crates/server : Backend API server + theme loader

## How to run

1. Check `.env.example` of each project and official documentation of dependencies, make your own `.env`.
2. run sqlx migration
3. import themes
4. run server
5. run client

## Authentication

The client starts a local HTTP server on port 8080 to handle Discord OAuth callbacks from Supabase. After authentication, it fetches themes from the backend and submits votes.
