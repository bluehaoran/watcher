# Multi-stage Rust build  
FROM rust:1.89-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static \
    chromium \
    nss \
    freetype \
    harfbuzz \
    ca-certificates \
    ttf-freefont

WORKDIR /app

# Copy source code and build
COPY . .
RUN cargo build --release

# Runtime stage
FROM alpine:3.19
RUN apk add --no-cache \
    ca-certificates \
    chromium \
    nss \
    freetype \
    harfbuzz \
    ttf-freefont \
    libgcc

# Tell headless_chrome to use installed Chromium
ENV CHROME_PATH=/usr/bin/chromium-browser

WORKDIR /app

# Copy built binary
COPY --from=builder /app/target/release/uatu-watcher .

# Copy configuration and migrations
COPY --from=builder /app/migrations ./migrations/
COPY --from=builder /app/config ./config/
COPY --from=builder /app/static ./static/

# Create data directory
RUN mkdir -p /data

# Create non-root user
RUN addgroup -g 1001 -S appuser && \
    adduser -S appuser -u 1001 -G appuser

# Set ownership
RUN chown -R appuser:appuser /app /data

USER appuser

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:3000/health || exit 1

# Start application
CMD ["./uatu-watcher"]