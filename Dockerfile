# Frontend build stage (admin only - public frontend is pre-built by Makefile)
FROM node:22-alpine AS frontend-builder

WORKDIR /app

# Copy package files and install dependencies
COPY package*.json ./
RUN npm ci

# Copy vite config and admin frontend source
COPY vite.config.js ./
COPY admin-frontend ./admin-frontend

# Build admin frontend
RUN npm run build

# Rust build stage
FROM rust:alpine3.22 AS builder

WORKDIR /app

# Install build dependencies for Alpine/musl compatibility
RUN apk update
RUN apk add --no-cache musl-dev

# Copy manifest files
COPY Cargo.toml Cargo.lock ./

# Copy workspace crates
COPY crates ./crates

# Copy source code
COPY src ./src

# Build the release binary (with loadtest feature for load testing builds)
ARG BUILD_FEATURES=""
RUN if [ -n "$BUILD_FEATURES" ]; then \
    cargo build --release --features "$BUILD_FEATURES"; \
    else \
    cargo build --release; \
    fi

# Runtime stage
FROM alpine:3.22.2

WORKDIR /app

# Install minimal runtime dependencies
RUN apk add --no-cache ca-certificates

# Copy the binary from builder stage
COPY --from=builder /app/target/release/cavebatsofware-site-template ./cavebatsofware-site-template

# Copy built admin frontend from frontend-builder stage
COPY --from=frontend-builder /app/admin-assets ./admin-assets

# Copy pre-built public frontend (built by Makefile before docker build)
COPY public-assets ./public-assets

# Copy static assets
COPY assets ./assets
COPY landing.html ./landing.html
COPY entrypoint.sh ./entrypoint.sh

# Create non-root user (Alpine style)
RUN adduser -D -s /bin/false appuser && \
    chown -R appuser:appuser /app && \
    chmod +x ./entrypoint.sh

USER appuser

EXPOSE 3000

# Start the application
ENTRYPOINT ["./entrypoint.sh"]
