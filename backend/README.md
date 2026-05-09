# glum-backend

gRPC backend for the glum notes and todos app. Built with Rust, Axum, tonic, and SQLx. Runs on PostgreSQL.

## Stack

- **[tonic](https://github.com/hyperium/tonic)** — gRPC framework
- **[SQLx](https://github.com/launchbadge/sqlx)** — async PostgreSQL driver with compile-time query checking
- **[tokio](https://tokio.rs)** — async runtime

## Prerequisites

- Rust
- Docker
- kind
- mise
- sqlx-cli (`cargo install sqlx-cli --no-default-features --features postgres`)
- buf (`yay -S buf`)

## Development

### Start the local cluster

```bash
mise run cluster:up
```

### Run migrations

```bash
mise run migrate
```

### Build

```bash
mise run build
```

### Run locally

```bash
mise run dev
```

### Load image into kind

```bash
mise run load-image
```

### Tear down cluster

```bash
mise run cluster:down
```

## Structure

```
backend/
├── src/
│   ├── main.rs        # server setup
│   ├── notes.rs       # Notes gRPC service implementation
│   └── todos.rs       # Todos gRPC service implementation
├── protobufs/
│   ├── notes.proto    # Notes service definition
│   └── todo.proto     # Todos service definition
├── migrations/        # SQLx migrations
├── k8s/               # Kubernetes manifests
├── build.rs           # compiles proto files
└── Cargo.toml
```

## API

Runs on port `50051`. Services defined in `protobufs/`.

### Notes

- `CreateNote` — title (1–100 chars), content (1–10000 chars)
- `GetNote` — by UUID
- `ListNotes` — all notes
- `UpdateNote` — by UUID
- `DeleteNote` — by UUID

### Todos

- `CreateTodo` — title (1–100 chars), priority (LOW/MEDIUM/HIGH), optional due date
- `GetTodo` — by UUID
- `ListTodos` — all todos
- `UpdateTodo` — by UUID
- `ToggleTodo` — toggle completed by UUID
- `DeleteTodo` — by UUID
