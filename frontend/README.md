# Frontend

Web UI for flechons. Not scaffolded yet.

It will talk to the server (`crates/server`) through `/api`: list grids with `GET /api/grids`,
fetch one with `GET /api/grids/{name}`, and fill it with `POST /api/solve`.
